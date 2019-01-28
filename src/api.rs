use std::fmt;
use url::Url;

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
	client: reqwest::Client,
	username: String,
	auth_token: String,
}

impl CheckinAPI {
	fn get_base_url() -> Url {
		let URL = if cfg!(debug_assertions) {
			"https://checkin.dev.hack.gt"
		}
		else {
			"https://checkin.hack.gt"
		};
		Url::parse(URL).expect("Invalid base URL configured")
	}

	pub fn login(username: &str, password: &str) -> Result<Self, Error> {
		let client = reqwest::Client::new();
		let base_url = CheckinAPI::get_base_url();

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
			Some(token) => {
				Ok(Self {
					client,
					username: username.to_owned(),
					auth_token: token,
				})
			},
			None => Err("No auth token set by server".into())
		}
	}
}
