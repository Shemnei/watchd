#![feature(io_error_other)]

use std::fmt;
use std::io::Read;
use std::path::Path;

use serde::de::{self, Deserializer, Visitor};
use serde::ser::Serializer;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SimpleDate {
	year: u16,
	month: u8,
	day: u8,
}

impl Serialize for SimpleDate {
	fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
	where
		S: Serializer,
	{
		serializer.serialize_str(&format!(
			"{:04}-{:02}-{:02}",
			self.year, self.month, self.day
		))
	}
}

struct SimpleDateVisitor;

impl<'de> Visitor<'de> for SimpleDateVisitor {
	type Value = SimpleDate;

	fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
		formatter.write_str(
			"a date formatted according to iso 8601 standard (yyyy-mm-dd)",
		)
	}

	fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
	where
		E: de::Error,
	{
		let mut parts = v.splitn(3, '-');

		let year = parts
			.next()
			.ok_or_else(|| E::custom("invalid date format"))?
			.parse()
			.map_err(|_| E::custom("year part not an integer"))?;

		let month = parts
			.next()
			.ok_or_else(|| E::custom("invalid date format"))?
			.parse()
			.map_err(|_| E::custom("month part not an integer"))?;

		let day = parts
			.next()
			.ok_or_else(|| E::custom("invalid date format"))?
			.parse()
			.map_err(|_| E::custom("day part not an integer"))?;

		Ok(SimpleDate { year, month, day })
	}
}

impl<'de> Deserialize<'de> for SimpleDate {
	fn deserialize<D>(deserializer: D) -> Result<SimpleDate, D::Error>
	where
		D: Deserializer<'de>,
	{
		deserializer.deserialize_str(SimpleDateVisitor)
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnimeKind {
	Tv,
	Movie,
	Ova,
	Ona,
	Special,
	Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AnimeStatus {
	Finished,
	Ongoing,
	Upcoming,
	Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Season {
	Spring,
	Summer,
	Fall,
	Winter,
	Undefined,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct License {
	name: String,
	url: Url,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnimeSeason {
	season: Season,
	year: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Anime {
	sources: Vec<Url>,
	title: String,
	#[serde(rename = "type")]
	kind: AnimeKind,
	episodes: u32,
	status: AnimeStatus,
	anime_season: AnimeSeason,
	picture: Url,
	thumbnail: Url,
	synonyms: Vec<String>,
	relations: Vec<Url>,
	tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Database {
	license: License,
	repository: Url,
	last_update: SimpleDate,
	data: Vec<Anime>,
}

impl Database {
	pub fn load(path: impl AsRef<Path>) -> Result<Self, std::io::Error> {
		fn _load(path: &Path) -> Result<Database, std::io::Error> {
			let file = std::fs::File::open(path)?;

			// simd_json: 450 ms
			#[cfg(feature = "simd-json")]
			{
				simd_json::from_reader(file)
					.map_err(|err| std::io::Error::other(err))
			}

			// serde_json: 25 s
			#[cfg(not(feature = "simd-json"))]
			{
				serde_json::from_reader(file)
					.map_err(|err| std::io::Error::other(err))
			}
		}

		_load(path.as_ref())
	}

	pub fn from_reader(r: impl Read) -> Result<Self, std::io::Error> {
		// simd_json: 450 ms
		#[cfg(feature = "simd-json")]
		{
			simd_json::from_reader(r).map_err(|err| std::io::Error::other(err))
		}

		// serde_json: 25 s
		#[cfg(not(feature = "simd-json"))]
		{
			serde_json::from_reader(r)
				.map_err(|err| std::io::Error::other(err))
		}
	}
}

#[cfg(feature = "fetch")]
mod fetch_shared {
	pub(crate) const DATABASE_URL: &'static str = "https://github.com/manami-project/anime-offline-database/raw/master/anime-offline-database-minified.json";
}

#[cfg(feature = "fetch")]
pub mod fetch {
	use std::io::Write;

	use crate::fetch_shared::DATABASE_URL;
	use crate::Database;

	#[derive(Debug, thiserror::Error)]
	pub enum Error {
		#[error("Request failed: `{0}`")]
		RequestError(#[from] ureq::Error),
		#[error("Io operation failed: `{0}`")]
		IoError(#[from] std::io::Error),
	}

	impl Database {
		pub fn fetch(mut w: impl Write) -> Result<u64, Error> {
			let mut reader = ureq::get(DATABASE_URL).call()?.into_reader();
			std::io::copy(&mut reader, &mut w).map_err(|err| err.into())
		}
	}
}

#[test]
fn db_read() -> anyhow::Result<()> {
	use std::time::Instant;

	let it = Instant::now();
	let db = Database::load(
		"assets/anime-offline-database/anime-offline-database.json",
	)?;
	let el = it.elapsed();

	println!("Took: {:?}", el);

	println!("Animes: {}", db.data.len());

	Ok(())
}

#[test]
#[cfg(feature = "fetch")]
fn db_fetch_read() -> anyhow::Result<()> {
	use std::time::Instant;

	// ~31 MB
	let mut buffer = Vec::with_capacity(40 * 1024 * 1024);
	let bread = Database::fetch(&mut buffer).unwrap();

	println!("BRead: {bread}");

	let it = Instant::now();
	let db = Database::from_reader(&buffer[..])?;
	let el = it.elapsed();

	println!("Took: {:?}", el);

	println!("Animes: {}", db.data.len());

	Ok(())
}
