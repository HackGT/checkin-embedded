use rppal::i2c::I2c;
use rppal::i2c::Result;
use rppal::gpio::Gpio;
use std::{ thread, time };
use std::sync::{ Arc, Mutex };

pub struct HT16K33 {
	device: I2c,
	buffer: [u8; 16],
}

const HT16K33_BLINK_CMD: u8       = 0x80;
const HT16K33_BLINK_DISPLAYON: u8 = 0x01;
const HT16K33_BLINK_OFF: u8       = 0x00;
const HT16K33_BLINK_2HZ: u8       = 0x02;
const HT16K33_BLINK_1HZ: u8       = 0x04;
const HT16K33_BLINK_HALFHZ: u8    = 0x06;
const HT16K33_SYSTEM_SETUP: u8    = 0x20;
const HT16K33_OSCILLATOR: u8      = 0x01;
const HT16K33_CMD_BRIGHTNESS: u8  = 0xE0;

struct Char(u8, u64);
impl Char {
	fn from(c: char) -> Char {
		match c {
			'!' => Char(1, 0b0101111100000000000000000000000000000000),
			'"' => Char(3, 0b0000001100000000000000110000000000000000),
			'#' => Char(5, 0b0001010000111110000101000011111000010100),
			'$' => Char(4, 0b0010010001101010001010110001001000000000),
			'%' => Char(5, 0b0110001100010011000010000110010001100011),
			'&' => Char(5, 0b0011011001001001010101100010000001010000),
			'\'' => Char(1, 0b0000001100000000000000000000000000000000),
			'(' => Char(3, 0b0001110000100010010000010000000000000000),
			')' => Char(3, 0b0100000100100010000111000000000000000000),
			'*' => Char(5, 0b0010100000011000000011100001100000101000),
			'+' => Char(5, 0b0000100000001000001111100000100000001000),
			',' => Char(2, 0b1011000001110000000000000000000000000000),
			'-' => Char(4, 0b0000100000001000000010000000100000000000),
			'.' => Char(2, 0b0110000001100000000000000000000000000000),
			'/' => Char(4, 0b0110000000011000000001100000000100000000),
			'0' => Char(4, 0b0011111001000001010000010011111000000000),
			'1' => Char(3, 0b0100001001111111010000000000000000000000),
			'2' => Char(4, 0b0110001001010001010010010100011000000000),
			'3' => Char(4, 0b0010001001000001010010010011011000000000),
			'4' => Char(4, 0b0001100000010100000100100111111100000000),
			'5' => Char(4, 0b0010011101000101010001010011100100000000),
			'6' => Char(4, 0b0011111001001001010010010011000000000000),
			'7' => Char(4, 0b0110000100010001000010010000011100000000),
			'8' => Char(4, 0b0011011001001001010010010011011000000000),
			'9' => Char(4, 0b0000011001001001010010010011111000000000),
			':' => Char(2, 0b0101000000000000000000000000000000000000),
			';' => Char(2, 0b1000000001010000000000000000000000000000),
			'<' => Char(3, 0b0001000000101000010001000000000000000000),
			'=' => Char(3, 0b0001010000010100000101000000000000000000),
			'>' => Char(3, 0b0100010000101000000100000000000000000000),
			'?' => Char(4, 0b0000001001011001000010010000011000000000),
			'@' => Char(5, 0b0011111001001001010101010101110100001110),
			'A' => Char(4, 0b0111111000010001000100010111111000000000),
			'B' => Char(4, 0b0111111101001001010010010011011000000000),
			'C' => Char(4, 0b0011111001000001010000010010001000000000),
			'D' => Char(4, 0b0111111101000001010000010011111000000000),
			'E' => Char(4, 0b0111111101001001010010010100000100000000),
			'F' => Char(4, 0b0111111100001001000010010000000100000000),
			'G' => Char(4, 0b0011111001000001010010010111101000000000),
			'H' => Char(4, 0b0111111100001000000010000111111100000000),
			'I' => Char(3, 0b0100000101111111010000010000000000000000),
			'J' => Char(4, 0b0011000001000000010000010011111100000000),
			'K' => Char(4, 0b0111111100001000000101000110001100000000),
			'L' => Char(4, 0b0111111101000000010000000100000000000000),
			'M' => Char(5, 0b0111111100000010000011000000001001111111),
			'N' => Char(5, 0b0111111100000100000010000001000001111111),
			'O' => Char(4, 0b0011111001000001010000010011111000000000),
			'P' => Char(4, 0b0111111100001001000010010000011000000000),
			'Q' => Char(4, 0b0011111001000001010000011011111000000000),
			'R' => Char(4, 0b0111111100001001000010010111011000000000),
			'S' => Char(4, 0b0100011001001001010010010011001000000000),
			'T' => Char(5, 0b0000000100000001011111110000000100000001),
			'U' => Char(4, 0b0011111101000000010000000011111100000000),
			'V' => Char(5, 0b0000111100110000010000000011000000001111),
			'W' => Char(5, 0b0011111101000000001110000100000000111111),
			'X' => Char(5, 0b0110001100010100000010000001010001100011),
			'Y' => Char(5, 0b0000011100001000011100000000100000000111),
			'Z' => Char(4, 0b0110000101010001010010010100011100000000),
			'[' => Char(2, 0b0111111101000001000000000000000000000000),
			'\\' => Char(4, 0b0000000100000110000110000110000000000000),
			']' => Char(2, 0b0100000101111111000000000000000000000000),
			'^' => Char(3, 0b0000001000000001000000100000000000000000),
			'_' => Char(4, 0b0100000001000000010000000100000000000000),
			'`' => Char(2, 0b0000000100000010000000000000000000000000),
			'a' => Char(4, 0b0010000001010100010101000111100000000000),
			'b' => Char(4, 0b0111111101000100010001000011100000000000),
			'c' => Char(4, 0b0011100001000100010001000010100000000000),
			'd' => Char(4, 0b0011100001000100010001000111111100000000),
			'e' => Char(4, 0b0011100001010100010101000001100000000000),
			'f' => Char(3, 0b0000010001111110000001010000000000000000),
			'g' => Char(4, 0b1001100010100100101001000111100000000000),
			'h' => Char(4, 0b0111111100000100000001000111100000000000),
			'i' => Char(3, 0b0100010001111101010000000000000000000000),
			'j' => Char(4, 0b0100000010000000100001000111110100000000),
			'k' => Char(4, 0b0111111100010000001010000100010000000000),
			'l' => Char(3, 0b0100000101111111010000000000000000000000),
			'm' => Char(5, 0b0111110000000100011111000000010001111000),
			'n' => Char(4, 0b0111110000000100000001000111100000000000),
			'o' => Char(4, 0b0011100001000100010001000011100000000000),
			'p' => Char(4, 0b1111110000100100001001000001100000000000),
			'q' => Char(4, 0b0001100000100100001001001111110000000000),
			'r' => Char(4, 0b0111110000001000000001000000010000000000),
			's' => Char(4, 0b0100100001010100010101000010010000000000),
			't' => Char(3, 0b0000010000111111010001000000000000000000),
			'u' => Char(4, 0b0011110001000000010000000111110000000000),
			'v' => Char(5, 0b0001110000100000010000000010000000011100),
			'w' => Char(5, 0b0011110001000000001111000100000000111100),
			'x' => Char(5, 0b0100010000101000000100000010100001000100),
			'y' => Char(4, 0b1001110010100000101000000111110000000000),
			'z' => Char(3, 0b0110010001010100010011000000000000000000),
			'{' => Char(3, 0b0000100000110110010000010000000000000000),
			'|' => Char(1, 0b0111111100000000000000000000000000000000),
			'}' => Char(3, 0b0100000100110110000010000000000000000000),
			'~' => Char(4, 0b0000100000000100000010000000010000000000),
			_ => Char(3, 0x0) // Space
		}
	}
}

impl HT16K33 {
	pub fn new(address: u8) -> Result<Self> {
		let mut device = I2c::new()?;
		device.set_slave_address(address as u16)?;
		// Turn on the oscillator
		device.block_write(HT16K33_SYSTEM_SETUP | HT16K33_OSCILLATOR, &[])?;
		// Turn display on with no blinking
        device.block_write(HT16K33_BLINK_CMD | HT16K33_BLINK_DISPLAYON | HT16K33_BLINK_OFF, &[])?;

		let mut instance = Self {
			device,
			buffer: [0; 16],
		};

		// Set display to full brightness
        HT16K33::set_brightness(&instance, 15)?;

		HT16K33::clear(&mut instance)?;
		Ok(instance)
	}

	pub fn scroll_text(text: &str, devices: &mut [&mut HT16K33], millis_per_column: u64) -> Result<()> {
		let mut columns: Vec<u8> = Vec::new();

		for character in text.chars() {
			let c = Char::from(character);

			for x in ((5 - c.0)..5).rev() {
				let row = ((c.1 >> x * 8) & 0xFF) as u8;

				columns.push(row);
			}
			if character != ' ' {
				// Add a blank column between characters
				columns.push(0x0);
			}
		}

		// From 0 to text length + screen length (text slides entirely past)
		for column_number in 0..columns.len() + 8 * devices.len() {
			let mut i = column_number as i32; // First column's (AKA start of string) location
			for column in columns.iter() {
				// If this piece of text is on-screen
				if i < 8 * devices.len() as i32 {
					for y in 0..8 {
						let device = (i / 8) as usize; // 8 columns per device
						let x = i % 8;
						let on = column & (1 << (7 - y)) != 0;
						devices[device].set_pixel(x as u8, y, on);
					}
				}
				i -= 1;
				if i < 0 {
					break;
				}
			}
			let after_text_column = column_number as i32 - columns.len() as i32;
			if after_text_column >= 0 {
				// Clear column after text has scrolled by
				for y in 0..8 {
					let device = (after_text_column / 8) as usize; // 8 columns per device
					let x = after_text_column % 8;
					devices[device].set_pixel(x as u8, y, false);
				}
			}
			for device in devices.iter_mut() {
				device.display_buffer()?;
			}
			thread::sleep(time::Duration::from_millis(millis_per_column));
		}
		Ok(())
	}

	pub fn all_on(&mut self) -> Result<()> {
		for x in 0..8 {
			for y in 0..8 {
				self.set_pixel(x, y, true);
			}
		}
		self.display_buffer()?;
		Ok(())
	}

	pub fn set_brightness(&self, level: u8) -> Result<()> {
		if level > 15 {
			panic!("Brightness must be a value of 0 to 15");
		}
		self.device.block_write(HT16K33_CMD_BRIGHTNESS | level, &[])
	}

	pub fn set_pixel(&mut self, x: u8, y: u8, on: bool) {
		if x > 7 || y > 7 {
			panic!("Pixel location out of range ({}, {})", x, y);
		}
		let led = y * 16 + ((x + 7) % 8);
		let pos = led / 8;
		let offset = led % 8;
		if on {
			self.buffer[pos as usize] |= 1 << offset;
		}
		else {
			self.buffer[pos as usize] &= !(1 << offset);
		}
	}

	pub fn display_buffer(&mut self) -> Result<()> {
		for i in 0..self.buffer.len() {
			let value = self.buffer[i];
			self.device.write(&[i as u8, value])?;
		}
		Ok(())
	}

	pub fn clear(&mut self) -> Result<()> {
		for x in 0..8 {
			for y in 0..8 {
				self.set_pixel(x, y, false);
			}
		}
		self.display_buffer()?;
		Ok(())
	}
}

pub struct Tone {
	frequency: f64,
	duration: time::Duration,
}
impl Tone {
	pub fn new(frequency: f64, millis: u64) -> Self {
		Self { frequency, duration: time::Duration::from_millis(millis) }
	}
}

/// Spawns threads to control peripherals without blocking the main thread
pub struct Notifier {
	success_display: Arc<Mutex<HT16K33>>,
	error_display: Arc<Mutex<HT16K33>>,
	buzzer: Arc<Mutex<rppal::gpio::OutputPin>>,
}
impl Notifier {
	pub fn start(success_display_address: u8, error_display_address: u8, buzzer_pin: u8) -> Self {
		let success_display = Arc::new(Mutex::new(HT16K33::new(success_display_address).unwrap()));
        let error_display = Arc::new(Mutex::new(HT16K33::new(error_display_address).unwrap()));

		let gpio = Gpio::new().unwrap();
		let buzzer = gpio.get(buzzer_pin).unwrap().into_output();
		let buzzer = Arc::new(Mutex::new(buzzer));

		Self {
			success_display,
			error_display,
			buzzer,
		}
	}

	pub fn scroll_text(&self, text: &str) {
		const SPEED: u64 = 5; // A speedy yet readable default
		self.scroll_text_speed(text, SPEED);
	}

	pub fn scroll_text_speed(&self, text: &str, millis_per_column: u64) {
		let success_display = Arc::clone(&self.success_display);
		let error_display = Arc::clone(&self.error_display);
		let text = text.to_owned();
		thread::spawn(move || {
			let mut success_display = success_display.lock().unwrap();
			let mut error_display = error_display.lock().unwrap();

			// List goes right to left
			HT16K33::scroll_text(&text, &mut [&mut *error_display, &mut *success_display], millis_per_column).unwrap();
		});
	}

	pub fn flash(&self, success: bool, duration: u64) {
		let display = if success { &self.success_display } else { &self.error_display };
		let display = Arc::clone(&display);
		thread::spawn(move || {
			let mut display = display.lock().unwrap();

			display.all_on().unwrap();
			thread::sleep(time::Duration::from_millis(duration));
			display.clear().unwrap();
		});
	}

	pub fn flash_multiple(&self, success: bool, durations: Vec<u64>) {
		let display = if success { &self.success_display } else { &self.error_display };
		let display = Arc::clone(&display);
		thread::spawn(move || {
			let mut display = display.lock().unwrap();

			let mut is_on = false;
			for duration in durations.iter() {
				if is_on {
					display.clear().unwrap();
				}
				else {
					display.all_on().unwrap();
				}
				is_on = !is_on;
				thread::sleep(time::Duration::from_millis(*duration));
			}
		});
	}

	pub fn beep(&self, tones: Vec<Tone>) {
		let buzzer = Arc::clone(&self.buzzer);
		thread::spawn(move || {
			let mut buzzer = buzzer.lock().unwrap();

			for tone in tones.iter() {
				let mut start = time::Instant::now();
				if tone.frequency == 0.0 {
					thread::sleep(tone.duration);
				}
				else {
					let microseconds = (1.0 / tone.frequency) * 1000.0 * 1000.0;

					while start.elapsed() < tone.duration {
						buzzer.set_high();
						thread::sleep(time::Duration::from_micros(microseconds as u64));
						buzzer.set_low();
						thread::sleep(time::Duration::from_micros(microseconds as u64));
					}
				}
			}
		});
	}
}
