use reqwest::Client;
use serde_derive::Deserialize;
#[allow(unused_imports)]
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use std::path::Path;
use inquire::{Text, Autocomplete, CustomUserError, autocompletion::Replacement};
use futures::stream::{FuturesUnordered, StreamExt};

const MODPACKS_DIR: &str = "modpacks/";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    prepare_paths();
    let _ = backup_mods();

    let mut modpacks = Modpacks::new();
    let _ = modpacks.fill();
    let autocomplete = modpacks.gen_autocomplete();
    let pack = Text::new("Modpack name:")
        .with_autocomplete(autocomplete)
        .with_placeholder("e.g. modgasm-pack")
        .prompt()?;
    let modpack = modpacks.find_by_name(pack).expect("no such modpack");

    let version = Text::new("Game version:")
        .with_placeholder("e.g. 1.20.1")
        .prompt()?;
    let mut url_futures = FuturesUnordered::new();
    
    for m in modpack.mods{
        if let Some(id) = m.get_id(){
            let url = get_url(&client, modpack.loader.clone() ,id.to_owned(), version.clone());
            url_futures.push(url);
        }
    }

    while let Some(url) = url_futures.next().await {
        println!("{url:?}");
    }

    Ok(())
}


fn backup_mods() -> Result<(), Box<dyn std::error::Error>>{
    let _ = fs::remove_dir_all("mods_cache/backup");
    let _ = fs::create_dir_all("mods_cache/backup");
    for entry in fs::read_dir("mods")?{
        let file = entry?.path();
        let _ = fs::copy(file, "mods_cache/backup/");
    }
    Ok(())
}




fn prepare_paths(){
    let _ = fs::create_dir_all(MODPACKS_DIR);
    let _ = fs::create_dir_all("mods_cache");
    let _ = fs::create_dir_all("mods");
}


async fn get_url(client: &Client, loader: String, id: String, version: String) -> Option<String> {
    let params = [
        ("loaders", json!([loader]).to_string()),
        ("game_versions", json!([version]).to_string())
    ];
    
    let compatible_versions = client
        .get(format!("https://api.modrinth.com/v2/project/{id}/version"))
        .query(&params)
        .send()
        .await.ok()?
        .json::<Value>()
        .await.ok()?;
    if compatible_versions.as_array().unwrap().is_empty(){
        return None;
    }

    let specific_version = client
        .get(format!("https://api.modrinth.com/v2/version/{}", compatible_versions[0]["id"].as_str().unwrap()))
        .send()
        .await.ok()?
        .json::<Value>()
        .await.ok()?;
    #[allow(clippy::manual_map)]
    if let Some(url) = specific_version["files"][0]["url"].as_str(){
        Some(url.to_string())
    }
    else {
        None
    }

}

#[allow(dead_code)]
fn get_modpack_by_filename(filename:&str) -> Modpack{
    let path = Path::new(MODPACKS_DIR).join(filename);
    let content = fs::read_to_string(path) 
        .unwrap_or(r#"
{
    "name": "invalid-modpack",
    "loader": "fabric",
    "mods": [
    ]
}
"#.to_string());
    let modpack = json5::from_str(&content)
        .unwrap_or(Modpack{
            name: "Error while decoding".to_string(), 
            loader: "fabric".to_string(), 
            mods: Vec::new()});
    #[allow(clippy::let_and_return)]
    modpack

}

fn get_modpack_by_path(path: PathBuf) -> Option<Modpack> {
    let content = std::fs::read_to_string(&path).ok()?;
    let modpack = json5::from_str(&content).ok()?;
    Some(modpack)
}

#[derive(Clone, Debug)]
struct ModpackAutocomplete {
    options: Vec<String>,
}

impl Autocomplete for ModpackAutocomplete {
    fn get_suggestions(&mut self, input: &str) -> Result<Vec<String>, CustomUserError> {
        Ok(self
            .options
            .iter()
            .filter(|opt| opt.contains(input))
            .cloned()
            .collect())
    }

    fn get_completion(
        &mut self,
        _input: &str,
        highlighted: Option<String>,
    ) -> Result<Replacement, CustomUserError> {
        if let Some(suggestion) = highlighted {
            Ok(Replacement::Some(suggestion))
        } else {
            Ok(Replacement::None)
        }
    }
}

#[derive(Debug)]
pub struct Modpacks{
    modpacks: Vec<Modpack>,
}
impl Modpacks {
    fn new() -> Self{
        Self { modpacks: Vec::new() }
    }
    fn fill(&mut self) -> Result<(), Box<dyn std::error::Error>>{
        let path = Path::new(&MODPACKS_DIR);
        for entry in fs::read_dir(path)?{
            let entry = entry?;
            let path = entry.path();
            if matches!(path.extension().and_then(|e| e.to_str()), Some("jsonc")) {
                if let Some(mp) = get_modpack_by_path(path) {
                    self.modpacks.push(mp);
                }
            }
        }
        Ok(())
    }
    fn gen_autocomplete(&self) -> ModpackAutocomplete{
        let names = self.modpacks.iter().map(|modpack| {
            modpack.name.clone()
        }).collect();
        ModpackAutocomplete { options: names }
    }
    fn find_by_name(&self, name: String) -> Option<Modpack>{
        self.modpacks.clone().into_iter().find(|modpack| modpack.name ==name)
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Modpack {
    pub name: String,
    pub loader: String,
    pub mods: Vec<ModEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModEntry {
    pub link: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub desc: Option<String>,
    #[serde(default = "default_required")]
    pub required: bool,
}
impl ModEntry {
    fn get_id(&self) -> Option<&str>{
        if let Some(id) = &self.id{
            Some(id.as_str())
        }
        else if let Some(link) = &self.link {
           link.split("/").last()
            
        }
        else {
            None
        }
    }
}

fn default_required() -> bool {
    true
}
