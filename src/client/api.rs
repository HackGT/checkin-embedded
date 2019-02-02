use std::fmt;
use url::Url;
use serde_json::Value;
use serde::{ Serialize, Deserialize };
use reqwest::header::{ HeaderName, HeaderValue };
use crate::crypto::Signer;


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

pub struct CheckinAPI {
	base_url: Url,
	client: reqwest::Client,
	auth_token: String,
}

impl CheckinAPI {
	#[cfg(debug_assertions)]
	fn base_url() -> &'static str {
		"https://checkin.dev.hack.gt"
	}
	#[cfg(not(debug_assertions))]
	fn base_url() -> &'static str {
		"https://checkin.hack.gt"
	}

	pub fn login(username: &str, password: &str) -> Result<Self, Error> {
		let client = reqwest::Client::new();
		let base_url = Url::parse(CheckinAPI::base_url()).expect("Invalid base URL configured");

		let params = [("username", username), ("password", password)];
		let response = client.post(base_url.join("/api/user/login").unwrap())
			.form(&params)
			.send()?;

		if !response.status().is_success() {
			return Err("Invalid username or password".into());
		}

		let cookies = response.headers().get_all(reqwest::header::SET_COOKIE);
		let mut auth_token: Option<String> = None;
		let auth_regex = regex::Regex::new(r"^auth=(?P<token>[a-f0-9]+);").unwrap();
		for cookie in cookies.iter() {
			if let Ok(cookie) = cookie.to_str() {
				if let Some(capture) = auth_regex.captures(cookie) {
					auth_token = Some(capture["token"].to_owned());
				}
			}
		}

		match auth_token {
			Some(mut token) => {
				// Create a HTTP cookie header out of this token
				token.insert_str(0, "auth=");
				Ok(Self {
					base_url,
					client,
					auth_token: token,
				})
			},
			None => Err("No auth token set by server".into())
		}
	}

	pub fn from_token(auth_token: String) -> Self {
		let client = reqwest::Client::new();
		let base_url = Url::parse(CheckinAPI::base_url()).expect("Invalid base URL configured");
		Self { base_url, client, auth_token }
	}

	fn checkin_action(&self, check_in: bool, uuid: &str, tag: &str) -> Result<String, Error> {
		let action = if check_in { "check_in" } else { "check_out" };
		let query = format!(
			"mutation($user: ID!, $tag: String!) {{
				{}(user: $user, tag: $tag) {{
					user {{
						name
					}}
				}}
			}}
		", action);
		let body = json!({
			"query": query,
			"variables": {
				"user": uuid,
				"tag": tag,
			}
		});
		let response = self.client.post(self.base_url.join("/graphql").unwrap())
			.header(reqwest::header::COOKIE, self.auth_token.as_str())
			.json(&body)
			.send()?
			.text()?;
		let response: Value = serde_json::from_str(&response).or(Err("Invalid JSON response"))?;

		let pointer = format!("/data/{}/user/name", action);
		match response.pointer(&pointer) {
			Some(name) => Ok(name.as_str().unwrap().to_owned()),
			None => Err("Check in failed. Non-existent user ID? Not logged in?".into()),
		}
	}

	pub fn check_in(&self, uuid: &str, tag: &str) -> Result<String, Error> {
		self.checkin_action(true, uuid, tag)
	}
	pub fn check_out(&self, uuid: &str, tag: &str) -> Result<String, Error> {
		self.checkin_action(false, uuid, tag)
	}
}

#[cfg(test)]
mod checkin_api_tests {
	use super::CheckinAPI;

	#[test]
	fn login() {
		let username = std::env::var("USERNAME").unwrap();
		let password = std::env::var("PASSWORD").unwrap();

		let instance = CheckinAPI::login(&username, &password).unwrap();
		assert!(instance.auth_token.len() == 64);
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
	Authorized,
}

pub struct ManagerAPI {
	base_url: Url,
	client: reqwest::Client,
	signer: Signer,
}

impl ManagerAPI {
	#[cfg(debug_assertions)]
	fn base_url() -> &'static str {
		"http://localhost:3000"
	}
	#[cfg(not(debug_assertions))]
	fn base_url() -> &'static str {
		"https://manager.checkin.hack.gt"
	}

	pub fn new() -> Self {
		let client = reqwest::Client::new();
		let base_url = Url::parse(ManagerAPI::base_url()).expect("Invalid base URL configured");
		Self { base_url, client, signer: Signer::load() }
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
		Ok(dbg!(response.status))
	}
}
