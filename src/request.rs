use std::io::prelude::*;

use flate2::bufread::GzDecoder;
use serde::Deserialize;

fn permalink_for<S: AsRef<str>>(seed: S) -> String {
    format!("https://alttpr.com/en/h/{}", seed.as_ref())
}

fn meta_json_uri_for<S: AsRef<str>>(seed: S) -> String {
    format!(
        "https://alttpr-patch-data.s3.us-east-2.amazonaws.com/{}.json",
        seed.as_ref()
    )
}

#[derive(Deserialize, Debug)]
pub struct SeedJson {
    pub spoiler: Spoiler,
}

#[derive(Deserialize, Debug)]
pub struct Spoiler {
    pub meta: MetaData,
}

#[derive(Deserialize, Debug)]
pub struct MetaData {
    pub goal: String,
    pub mode: String,
    pub build: String,
    pub logic: String,
    pub worlds: usize,
    pub weapons: String,
    pub rom_mode: String,
    pub spoilers: String,
    pub world_id: usize,
    pub item_pool: String,
    pub tournament: bool,
    pub accessibility: String,
    pub dungeon_items: String,
    pub item_placement: String,
    pub allow_quickswap: bool,
    pub item_functionality: String,
    #[serde(rename = "enemizer.pot_shuffle")]
    pub enemizer_pot_shuffle: String,
    #[serde(rename = "enemizer.boss_shuffle")]
    pub enemizer_boss_shuffle: String,
    #[serde(rename = "enemizer.enemy_damage")]
    pub enemizer_enemy_damage: String,
    #[serde(rename = "enemizer.enemy_health")]
    pub enemizer_enemy_health: String,
    #[serde(rename = "enemizer.enemy_shuffle")]
    pub enemizer_enemy_shuffle: String,
    pub entry_crystals_ganon: String,
    pub entry_crystals_tower: String,
}

pub fn fetch_metadata_for<S: AsRef<str>>(seed: S) -> anyhow::Result<(String, SeedJson)> {
    let response = reqwest::blocking::get(meta_json_uri_for(&seed))?;
    let r = &response.bytes()?.to_vec()[..];
    let mut d = GzDecoder::new(r);
    let mut s = String::new();
    d.read_to_string(&mut s)?;

    Ok((permalink_for(seed), serde_json::from_str(&s)?))
}
