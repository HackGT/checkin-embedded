use rocket::{ Outcome, State };
use rocket::http::{ Cookie, Cookies, Status };
use rocket::request::{ self, Request, FromRequest, Form };
use rocket::response::Redirect;
use rocket_contrib::templates::Template;
use wither::model::Model;
use hackgt_nfc::api::CheckinAPI;
use crate::DB;
use crate::models::User;

pub struct AuthenticatedUser(User);

impl std::ops::Deref for AuthenticatedUser {
    type Target = User;
    #[inline(always)]
    fn deref(&self) -> &User {
        &self.0
    }
}

#[derive(Debug)]
pub enum AuthenticatedUserError {
    Missing,
    Invalid,
    DBError(wither::mongodb::error::Error),
}

impl<'a, 'r> FromRequest<'a, 'r> for AuthenticatedUser {
    type Error = AuthenticatedUserError;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Self::Error> {
        let cookies = request.cookies();
		let token = cookies.get("auth").map(|cookie| cookie.value());
        match token {
            Some(token) => {
                let db = request.guard::<State<DB>>().unwrap();
                let user = User::find_one(db.clone(), Some(doc!{ "auth_token": token }), None);
                match user {
                    Ok(Some(user)) => Outcome::Success(AuthenticatedUser(user)),
                    Ok(None) => Outcome::Failure((Status::Forbidden, AuthenticatedUserError::Invalid)),
                    Err(err) => Outcome::Failure((Status::InternalServerError, AuthenticatedUserError::DBError(err))),
                }
            },
            None => {
                Outcome::Failure((Status::Unauthorized, AuthenticatedUserError::Missing))
            },
        }
    }
}

#[get("/login")]
pub fn login() -> Template {
    Template::render("login", &json!({}))
}

#[derive(FromForm, Debug)]
pub struct LoginInfo {
    username: String,
    password: String,
}
#[post("/login", data = "<body>")]
pub fn process_login(body: Form<LoginInfo>, mut cookies: Cookies, db: State<DB>) -> Redirect {
    match CheckinAPI::login(&body.username, &body.password) {
        Ok(api) => {
            let token = api.auth_token();
            let mut user = User {
                id: None,
                username: body.username.clone(),
                auth_token: token.to_owned(),
            };
            user.save(db.clone(), None).unwrap();
            cookies.add(
                Cookie::build("auth", token.to_owned())
                .path("/")
                .secure(!cfg!(debug_assertions)) // Will be secure-only when built with --release
                .http_only(true)
                .finish()
            );
            Redirect::to("/")
        },
        Err(err) => {
            eprintln!("{:?}", err);
            Redirect::to("/auth/login")
        }
    }
}

#[catch(401)]
pub fn unauthorized_redirect(_req: &Request) -> Redirect {
    Redirect::to("/auth/login")
}
