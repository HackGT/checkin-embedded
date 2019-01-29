use std::fmt;
use url::Url;
use serde_json::Value;

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
	auth_token: String,
}

impl CheckinAPI {
	fn get_base_url() -> Url {
		let url = if cfg!(debug_assertions) {
			"https://checkin.dev.hack.gt"
		}
		else {
			"https://checkin.hack.gt"
		};
		Url::parse(url).expect("Invalid base URL configured")
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
			Some(mut token) => {
				// Create a HTTP cookie header out of this token
				token.insert_str(0, "auth=");
				Ok(Self {
					client,
					auth_token: token,
				})
			},
			None => Err("No auth token set by server".into())
		}
	}

	pub fn from_token(auth_token: String) -> Self {
		let client = reqwest::Client::new();
		Self { client, auth_token }
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
		let response = self.client.post(CheckinAPI::get_base_url().join("/graphql").unwrap())
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
