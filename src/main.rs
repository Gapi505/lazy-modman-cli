use reqwest::Client;
use serde_derive::Deserialize;
#[allow(unused_imports)]
use serde::Deserialize;
use serde_json::{Value, json};
use std::fmt::Display;
use std::io::Write;
use std::{fs, path::PathBuf};
use std::path::Path;
use inquire::{Text, Autocomplete, CustomUserError, autocompletion::Replacement, ui::{RenderConfig, StyleSheet, Color, Styled}};
use futures::stream::{FuturesOrdered, StreamExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    inquire::set_global_render_config(get_render_config());

    prepare_paths();

    println!(r#"
 ██▓    ▄▄▄      ▒███████▒▓██   ██▓    ███▄ ▄███▓ ▒█████  ▓█████▄  ███▄ ▄███▓ ▄▄▄       ███▄    █ 
▓██▒   ▒████▄    ▒ ▒ ▒ ▄▀░ ▒██  ██▒   ▓██▒▀█▀ ██▒▒██▒  ██▒▒██▀ ██▌▓██▒▀█▀ ██▒▒████▄     ██ ▀█   █ 
▒██░   ▒██  ▀█▄  ░ ▒ ▄▀▒░   ▒██ ██░   ▓██    ▓██░▒██░  ██▒░██   █▌▓██    ▓██░▒██  ▀█▄  ▓██  ▀█ ██▒
▒██░   ░██▄▄▄▄██   ▄▀▒   ░  ░ ▐██▓░   ▒██    ▒██ ▒██   ██░░▓█▄   ▌▒██    ▒██ ░██▄▄▄▄██ ▓██▒  ▐▌██▒
░██████▒▓█   ▓██▒▒███████▒  ░ ██▒▓░   ▒██▒   ░██▒░ ████▓▒░░▒████▓ ▒██▒   ░██▒ ▓█   ▓██▒▒██░   ▓██░
░ ▒░▓  ░▒▒   ▓▒█░░▒▒ ▓░▒░▒   ██▒▒▒    ░ ▒░   ░  ░░ ▒░▒░▒░  ▒▒▓  ▒ ░ ▒░   ░  ░ ▒▒   ▓▒█░░ ▒░   ▒ ▒ 
░ ░ ▒  ░ ▒   ▒▒ ░░░▒ ▒ ░ ▒ ▓██ ░▒░    ░  ░      ░  ░ ▒ ▒░  ░ ▒  ▒ ░  ░      ░  ▒   ▒▒ ░░ ░░   ░ ▒░
  ░ ░    ░   ▒   ░ ░ ░ ░ ░ ▒ ▒ ░░     ░      ░   ░ ░ ░ ▒   ░ ░  ░ ░      ░     ░   ▒      ░   ░ ░ 
    ░  ░     ░  ░  ░ ░     ░ ░               ░       ░ ░     ░           ░         ░  ░         ░ 
                 ░         ░ ░                             ░                                      
    "#);

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
    modpack.get_download_metadata(&client, version).await;

    /*for m in modpack.mods.iter(){
        println!("{:?}: {:?}", m.name, m.download_link);
    }*/

    modpack.downlaod_modpack().await;

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
    let _ = fs::create_dir_all("modpacks");
    let _ = fs::create_dir_all("mods_cache/backup");
    let _ = fs::create_dir_all("mods_cache/share");
    let _ = fs::create_dir_all("mods");
}

async fn download(url: String, path: PathBuf) -> Result<(), Box<dyn std::error::Error>> {
    let response = reqwest::get(url).await?;
    let content = response.bytes().await?;

    let mut downloaded_file = fs::File::create(&path)?;
    downloaded_file.write_all(&content)?;
    Ok(())
}

async fn get_file_metadata(client: &Client, loader: String, id: String, version: String) -> Option<(String, String)> {
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
    println!("{:#}", compatible_versions);
    if let Some(url) = specific_version["files"][0]["url"].as_str(){
        let filename = specific_version["files"][0]["filename"].as_str().unwrap().to_string();
        Some((url.to_string(), filename))
    }
    else {
        None
    }

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
        let path = Path::new("modpacks");
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
    pub version: Option<String>
}
impl Modpack {
    async fn get_download_metadata(&mut self, client: &Client, version: String){
        {
            let mut url_futures = FuturesOrdered::new();
            for m in self.mods.iter_mut(){
            if let Some(id) = m.get_id(){
                let url = m.get_compatible_versions(client, self.loader.clone() , version.clone());
                url_futures.push_back(url);
            }
            }
            while let Some(metadata) = url_futures.next().await{
                if metadata.is_none(){
                    continue;
                }
            }
        }
        for m in self.mods.iter_mut(){
            m.get_specific_version();
            m.get_download_params();
            let deps = m.get_dependencies();
            println!("{:#?}", deps);
        }
    }
    async fn downlaod_modpack(&self){
        for m in self.mods.iter(){
            if m.download_link.is_none() || m.filename.is_none(){
                println!("Skipping {} as it does not exist", m.get_id().unwrap());
                continue;
            }
            let path = m.download_and_cache().await;
            let mods_path = Path::new("mods").join( m.filename.clone().unwrap());
            let res = fs::copy(&path, mods_path);
            if res.is_err(){
                println!("Error coping file {res:?}");
                continue;
            }

            let name = if let Some(name) = m.name.clone() {name} else {path.to_string_lossy().into_owned()};
            
            println!("Sucessfully installed {name}")
        }
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModEntry {
    pub link: Option<String>,
    pub id: Option<String>,
    pub version_id: Option<String>,
    pub name: Option<String>,
    pub desc: Option<String>,
    #[serde(default = "default_required")]
    pub required: bool,
    pub download_link: Option<String>,
    pub filename: Option<String>,
    pub compatible_versions: Option<Value>,
    pub specific_version: Option<Value>
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

    fn check_if_cached(&self) -> Option<PathBuf>{
        if self.filename.is_none() {return None;}
        for entry in fs::read_dir("mods_cache/share").ok()?{
            let entry = entry.ok()?;
            let filename = entry.file_name();
            if filename.to_string_lossy() == self.filename.clone().unwrap(){
                return Some(entry.path());
            }
        }
        None
    }

    async fn download_and_cache(&self) -> PathBuf{
        for i in 0..5{
            let path = self.check_if_cached();
            if let Some(path) = path{
                return path;
            }
            
            println!("Downloading {:?}", self.download_link.clone().unwrap());
            let download_path = Path::new("mods_cache/share").join(Path::new(self.filename.clone().unwrap().as_str()));
            let result = download(self.download_link.clone().unwrap(), download_path).await;
            if result.is_err(){
                println!("failed download: {:?}, retry: {}", result, 5-i);
            }
        }
        
        eprintln!("Failed to download {} after 5 attempts", self.filename.clone().unwrap());
        std::process::exit(1);
    }
    async fn get_compatible_versions(&mut self, client: &Client, loader: String, version: String) -> Option<()>{
        let params = [
            ("loaders", json!([loader]).to_string()),
            ("game_versions", json!([version]).to_string())
        ];
        let compatible_versions = client
            .get(format!("https://api.modrinth.com/v2/project/{}/version", self.id.as_ref().unwrap()))
            .query(&params)
            .send()
            .await.ok()?
            .json::<Value>()
            .await.ok()?;
        self.compatible_versions = Some(compatible_versions);
        Some(())
    }
    fn get_specific_version(&mut self) -> Option<()>{
        let versions = self.compatible_versions.as_ref()?.as_array()?;
        if self.version_id.is_some(){
            let res = versions.iter().find(|v|{
                if v["id"].as_str() == self.version_id.as_deref() {
                    true
                }
                else {
                    false
                }
            });
            if res.is_some(){
                self.specific_version = res.cloned();
                return Some(());
            }
        }

        self.specific_version = Some(versions[0].clone());
        Some(())

    }
    fn get_download_params(&mut self) -> Option<(String, String)>{

        let specific_version = &self.specific_version.as_ref()?;
        if let Some(url) = specific_version["files"][0]["url"].as_str(){
            let filename = specific_version["files"][0]["filename"].as_str().unwrap().to_string();
            self.download_link = Some(url.to_string());
            self.filename = Some(filename.clone());
            Some((url.to_string(), filename))
        }
        else {
            None
        }
    }
    fn get_dependencies(&self) -> Vec<ModEntry>{
        let dependancies = self.compatible_versions.as_ref().unwrap()[0]["dependencies"].as_array().unwrap();
        let dep_mods: Vec<_> = dependancies.iter().map(|i| {
            println!("{:#}", i);
            let project_id = i["project_id"].as_str().unwrap().to_string();
            let version_id = i["version_id"].as_str();
            let mod_ent = ModEntry {
                id: Some(project_id),
                version_id: version_id.map(str::to_string),
                ..Default::default()
            };
            mod_ent
            })
            .collect();
        dep_mods
    }
}
impl Default for ModEntry {
    fn default() -> Self {
        Self { 
            link: None, 
            id: None, 
            version_id: None, 
            name: None, 
            desc: None, 
            required: false, 
            download_link:None, 
            filename: None, 
            compatible_versions: None,
            specific_version: None
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
