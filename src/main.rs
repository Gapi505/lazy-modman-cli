use reqwest::Client;
use serde_derive::Deserialize;
#[allow(unused_imports)]
use serde::Deserialize;
use serde_json::{Value, json};
use std::{fs, path::PathBuf};
use std::path::Path;
use inquire::{Text, Autocomplete, CustomUserError, autocompletion::Replacement, ui::{RenderConfig, StyleSheet, Color, Styled}};
use futures::stream::{FuturesOrdered, StreamExt};

const MODPACKS_DIR: &str = "modpacks/";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    inquire::set_global_render_config(get_render_config());

    prepare_paths();

    let mut modpacks = Modpacks::new();
    let _ = modpacks.fill();
    let autocomplete = modpacks.gen_autocomplete();
    let pack = Text::new("Modpack name:")
        .with_autocomplete(autocomplete)
        .prompt()?;
    let mut modpack = modpacks.find_by_name(pack).expect("no such modpack");

    let version = Text::new("Game version:")
        .with_placeholder("1.20.1")
        .prompt()?;

    let _ = backup_and_remove_mods();
    modpack.get_download_links(&client, version).await;

    for m in modpack.mods{
        println!("{:?}: {:?}", m.name, m.download_link);
    }

    Ok(())
}


fn backup_and_remove_mods() -> Result<(), Box<dyn std::error::Error>> {
    let _ = fs::remove_dir_all("mods_cache/backup");
    fs::create_dir_all("mods_cache/backup")?;

    for entry in fs::read_dir("mods")? {
        let entry = entry?;
        let src = entry.path();
        let filename = src.file_name().unwrap(); // get "sodium.jar"
        let dest = Path::new("mods_cache/backup").join(filename);
        fs::copy(&src, &dest)?; // don't ignore result here
        fs::remove_file(&src)?;
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
impl Modpack {
    async fn get_download_links(&mut self, client: &Client, version: String){
        let mut url_futures = FuturesOrdered::new();
        
        for m in self.mods.iter(){
            if let Some(id) = m.get_id(){
                let url = get_url(client, self.loader.clone() ,id.to_owned(), version.clone());
                url_futures.push_back(url);
            }
        }
        let mut i = 0;
        while let Some(url) = url_futures.next().await{
            self.mods[i].download_link = url;
            i+=1;
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModEntry {
    pub link: Option<String>,
    pub id: Option<String>,
    pub name: Option<String>,
    pub desc: Option<String>,
    #[serde(default = "default_required")]
    pub required: bool,
    pub download_link: Option<String>
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


fn get_render_config() -> RenderConfig<'static>{
    let mut config = RenderConfig::default();
    config.selected_option = Some(StyleSheet::new().with_fg(Color::LightRed));
    config.help_message = StyleSheet::new().with_fg(Color::LightRed);
    config.prompt_prefix = Styled::new("?").with_fg(Color::LightRed);
    config.answered_prompt_prefix = Styled::new(">").with_fg(Color::LightRed);
    config.option = StyleSheet::new().with_fg(Color::DarkGrey);
    config.highlighted_option_prefix = Styled::new(">").with_fg(Color::DarkRed);
    config.answer = StyleSheet::new().with_fg(Color::LightRed);
    config
}
