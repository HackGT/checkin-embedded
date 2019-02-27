use serde::{ Serialize, Deserialize };
use mongodb::{
	coll::options::IndexModel,
	oid::ObjectId,
};

#[derive(Model, Serialize, Deserialize)]
pub struct Device {
	#[serde(rename="_id", skip_serializing_if="Option::is_none")]
    pub id: Option<ObjectId>,

	#[model(index(index="dsc", unique="true"))]
	pub public_key: String,

	pub friendly_name: String,
	pub username: String,

	pub ip_address: String,

	pub authorized: bool,
	pub status_set_by: Option<String>,
	pub pending: bool,
	pub credentials_created: bool,

	pub current_tag: Option<String>,
}

#[derive(Model, Serialize, Deserialize)]
pub struct User {
	#[serde(rename="_id", skip_serializing_if="Option::is_none")]
    pub id: Option<ObjectId>,

	pub username: String,
	pub auth_token: String,
}
