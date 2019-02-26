use std::fs;
use rand::rngs::OsRng;
use ed25519_dalek::{ Keypair, Signature };

pub struct APICredentials {
	pub username: String,
	pub password: String,
}

pub struct Signer {
	keypair: Keypair,
}

impl Signer {
	pub fn load() -> Self {
		const KEY_NAME: &'static str = "./instance.key";
		let keypair = match fs::read(KEY_NAME) {
			Ok(file) => {
				Keypair::from_bytes(&file).expect("Invalid key")
			},
			Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => {
				let mut csprng = OsRng::new().unwrap();
				let key = Keypair::generate(&mut csprng);

				fs::write(KEY_NAME, &key.to_bytes()[..]).expect("Error writing key to file");
				key
			},
			Err(err) => panic!("There was an error opening the key file: {:?}", err),
		};
		Self { keypair }
	}

	pub fn sign(&self, message: &[u8]) -> Signature {
		self.keypair.sign(message)
	}

	pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
		self.keypair.verify(message, signature).is_ok()
	}

	pub fn get_public_key(&self) -> [u8; ed25519_dalek::PUBLIC_KEY_LENGTH] {
		self.keypair.public.to_bytes()
	}

	// API credentials are comprised of
	//     Username = SHA-256 of public key (so that names don't always start with the same characters)
	//     Password = SHA-512 of private key
	pub fn get_api_credentials(&self) -> APICredentials {
		APICredentials {
			username: crypto_hash::hex_digest(crypto_hash::Algorithm::SHA256, &self.keypair.public.to_bytes()),
			password: crypto_hash::hex_digest(crypto_hash::Algorithm::SHA512, &self.keypair.secret.to_bytes()),
		}
	}
}

impl std::clone::Clone for Signer {
	fn clone(&self) -> Self {
		Signer::load()
	}
}
