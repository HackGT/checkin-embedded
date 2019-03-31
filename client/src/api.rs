use std::fmt;
use std::{ thread, time };
use std::sync::{ Arc, RwLock };
use url::Url;
use serde::{ Serialize, Deserialize };
use reqwest::header::{ HeaderName, HeaderValue };
use crate::crypto::Signer;
use crate::peripherals::Notifier;

pub enum Error {
	Network(reqwest::Error),
	Message(&'static str),
}
impl fmt::Debug for Error {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Error::Network(err) => write!(f, "{:?}", err),
			Error::Message(s) => write!(f, "{}", s),
		}
	}
}
impl From<reqwest::Error> for Error {
	fn from(err: reqwest::Error) -> Error {
		Error::Network(err)
	}
}
impl From<&'static str> for Error {
	fn from(err: &'static str) -> Error {
		Error::Message(err)
	}
}

struct SignedRequest {
	body: String,
	header_name: HeaderName,
	header_value: HeaderValue,
}

#[derive(Deserialize, PartialEq, Debug)]
pub enum ManagedStatus {
	Pending,
	Unauthorized,
	AuthorizedHasCredentials,
	AuthorizedNoCredentials,
}
#[derive(Debug)]
pub struct CredentialResponse {
	pub success: bool,
	pub error: Option<String>,
	pub details: Option<String>,
}

#[derive(Clone)]
pub struct ManagerAPI {
	base_url: Url,
	client: reqwest::Client,
	signer: Signer,
	pub current_tag: Arc<RwLock<Option<String>>>,
}

impl ManagerAPI {
	#[cfg(debug_assertions)]
	fn base_url() -> &'static str {
		"http://192.168.1.15:3000"
	}
	#[cfg(not(debug_assertions))]
	fn base_url() -> &'static str {
		"https://manager.checkin.hack.gt"
	}

	pub fn new() -> Self {
		let client = reqwest::Client::new();
		let base_url = Url::parse(ManagerAPI::base_url()).expect("Invalid base URL configured");
		Self {
			base_url,
			client,
			signer: Signer::load(),
			current_tag: Arc::new(RwLock::new(None)),
		}
	}

	fn sign_request<T: Serialize + ?Sized>(&self, request: &T) -> SignedRequest {
		let body = serde_json::to_string_pretty(request).expect("Could not serialize object for signing");

		let signature = self.signer.sign(body.as_bytes());
		let header_value = format!("ed25519 {}/{}", hex::encode(&self.signer.get_public_key()), hex::encode(&signature.to_bytes()[..]));

		SignedRequest {
			body,
			header_name: reqwest::header::AUTHORIZATION,
			header_value: HeaderValue::from_str(&header_value).unwrap(),
		}
	}

	pub fn get_name(&self) -> String {
		crypto_hash::hex_digest(crypto_hash::Algorithm::SHA256, &self.signer.get_public_key())
	}

	pub fn initialize(&self) -> Result<ManagedStatus, Error> {
		#[derive(Serialize)]
		struct Request<'a> {
			username: &'a str,
		}
		#[derive(Deserialize)]
		struct Response {
			status: ManagedStatus,
		}
		let request = Request {
			username: &self.get_name(),
		};
		let signed_request = self.sign_request(&request);

		let response: Response = self.client.post(self.base_url.join("/api/initialize").unwrap())
			.header(signed_request.header_name, signed_request.header_value)
			.header(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
			.body(signed_request.body)
			.send()?
			.json()?;
		Ok(response.status)
	}

	pub fn create_credentials(&self) -> Result<CredentialResponse, Error> {
		#[derive(Serialize)]
		struct Request<'a> {
			username: &'a str,
			password: &'a str,
		}
		#[derive(Deserialize)]
		struct Response {
			success: Option<bool>,
			error: Option<String>,
			details: Option<String>,
		}
		let credentials = self.signer.get_api_credentials();
		let request = Request {
			username: &credentials.username,
			password: &credentials.password,
		};
		let signed_request = self.sign_request(&request);

		let response: Response = self.client.post(self.base_url.join("/api/credentials").unwrap())
			.header(signed_request.header_name, signed_request.header_value)
			.header(reqwest::header::CONTENT_TYPE, HeaderValue::from_static("application/json"))
			.body(signed_request.body)
			.send()?
			.json()?;
		Ok(CredentialResponse {
			success: response.success.unwrap_or(false),
			error: response.error,
			details: response.details,
		})
	}

	pub fn get_tag(&self) -> Result<Option<String>, Error> {
		#[derive(Serialize)]
		struct Request<'a> {
			username: &'a str,
		}
		#[derive(Deserialize)]
		struct Response {
			current: Option<String>,
			all: Vec<String>,
		}
		let credentials = self.signer.get_api_credentials();
		let request = Request {
			username: &credentials.username
		};
		let response: Response = self.client.get(self.base_url.join("/api/tag").unwrap())
			.query(&request)
			.send()?
			.json()?;
		Ok(response.current)
	}

	pub fn update_tag(&self, notifier: &Notifier) {
		let current_tag = Arc::clone(&self.current_tag);
		match self.get_tag() {
			Ok(Some(new_tag)) => {
				let mut tag = current_tag.write().unwrap();
				// Only update if changed
				if tag.is_none() || tag.as_ref().unwrap() != &new_tag {
					*tag = Some(new_tag);
					notifier.scroll_text(&format!("Using new tag: {}", tag.as_ref().unwrap()));
				}
				else if !tag.is_none() {
					notifier.scroll_text(&format!("Tag: {}", tag.as_ref().unwrap()));
				}
			},
			Ok(None) => {
				let mut tag = current_tag.write().unwrap();
				*tag = None;
				notifier.scroll_text_speed("No tag defined by manager", 15);
			},
			Err(err) => println!("Tag check: {:?}", err)
		}
	}

	pub fn start_polling_for_tag(&self, seconds: u64, notifier: Arc<Notifier>) {
		// Spawn a thread that checks for current check-in tag
		let thread_instance = self.clone();
		let current_tag = Arc::clone(&self.current_tag);
		thread::spawn(move || {
			loop {
				match ManagerAPI::get_tag(&thread_instance) {
					Ok(Some(new_tag)) => {
						let mut tag = current_tag.write().unwrap();
						// Only update if changed
						if tag.is_none() || tag.as_ref().unwrap() != &new_tag {
							*tag = Some(new_tag);
							notifier.scroll_text(&format!("Using tag: {}", tag.as_ref().unwrap()));
						}
					},
					Ok(None) => {
						let mut tag = current_tag.write().unwrap();
						// Only update if newly null
						if tag.is_some() {
							*tag = None;
							notifier.scroll_text_speed("No tag defined by manager", 15);
						}
					},
					Err(err) => println!("Tag check thread: {:?}", err)
				}
				thread::sleep(time::Duration::from_secs(seconds));
			}
		});
	}
}
