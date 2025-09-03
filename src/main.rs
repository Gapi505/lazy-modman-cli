use reqwest::Client;
use serde_derive::Deserialize;
#[allow(unused_imports)]
use serde::Deserialize;
use serde_json::{Value, json};
use std::io::Write;
use std::{fs, path::PathBuf};
use std::path::Path;
use inquire::{Text, Autocomplete, CustomUserError, autocompletion::Replacement, ui::{RenderConfig, StyleSheet, Color, Styled}};
use tokio::task::JoinSet;

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
    let latest_version = get_latest_version().await;

    let version = Text::new("Game version:")
        .with_placeholder(&latest_version)
        .with_default(&latest_version)
        .prompt()?;

    let _ = backup_and_remove_mods();
    modpack.get_download_metadata(&client, version).await;

    /*for m in modpack.mods.iter(){
        println!("{:?}: {:?}", m.name, m.download_link);
    }*/

    modpack.downlaod_modpack().await;

    Ok(())
}

async fn get_latest_version() -> String {
    reqwest::get("https://piston-meta.mojang.com/mc/game/version_manifest.json").await
        .expect("cannot reach mojang.com")
        .json::<Value>().await.unwrap()["latest"]["release"].as_str().unwrap().to_string()
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

/*async fn get_file_metadata(client: &Client, loader: String, id: String, version: String) -> Option<(String, String)> {
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

}*/


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
        let loader = self.loader.clone();
        let mut processed_mods: Vec<ModEntry> = Vec::new();
        let mut task_set = JoinSet::new();
        for m in self.mods.clone() {
            let version_clone = version.clone();
            let loader_clone = loader.clone();
            let client_clone = client.clone();
            task_set.spawn(async move {
                let mut m_to_process = m;
                let deps = m_to_process.get_metadata(&client_clone, version_clone, loader_clone).await;
                (m_to_process, deps)
            });
        }
        while let Some(res) = task_set.join_next().await {
            let (processed_mod, dependencies) = res.expect("getting mod metadata panicked");
            processed_mods.push(processed_mod);

            for m in dependencies {
                let dup_check = processed_mods.iter().find(|pm| {
                    pm.id == m.id
                });
                let dup_check_og = self.mods.iter().find(|pm| {
                    pm.id == m.id
                });
                if dup_check.is_some() || dup_check_og.is_some() {continue;}
                let version_clone = version.clone();
                let loader_clone = loader.clone();
                let client_clone = client.clone();
                task_set.spawn(async move {
                    let mut m_to_process = m;
                    let deps = m_to_process.get_metadata(&client_clone, version_clone, loader_clone).await;
                    (m_to_process, deps)
                });
            }
        }

        self.mods = processed_mods;
        self.purge_duplicates();
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
            
            println!("Sucessfully installed {} ({})", m.id.as_ref().unwrap(), name);
        }
    }
    fn purge_duplicates(&mut self){
        let mut unique_mods = std::collections::HashSet::new();
        self.mods.retain(|m|{
            let id = m.id.clone().unwrap();
            let is_first = unique_mods.contains(&id.clone());
            unique_mods.insert(id.clone());
            !is_first
        });

    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct ModEntry {
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
            .get(format!("https://api.modrinth.com/v2/project/{}/version", self.get_id().unwrap()))
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
        if versions.is_empty(){
            return None;
        }
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
        if self.specific_version.is_none(){
            return Vec::new();
        }
        let dependancies = self.specific_version.as_ref().unwrap()["dependencies"].as_array().unwrap();
        let dep_mods: Vec<_> = dependancies.iter().map(|i| {
            let project_id = i["project_id"].as_str().unwrap().to_string();
            let version_id = i["version_id"].as_str();
            let required = i["dependency_type"].as_str().unwrap() == "required";
            let mod_ent = ModEntry {
                id: Some(project_id),
                version_id: version_id.map(str::to_string),
                required,
                ..Default::default()
            };
            mod_ent
            })
            .filter(|me| me.required)
            .collect();
        dep_mods
    }
    async fn get_metadata(&mut self, client: &Client, version: String, loader: String) -> Vec<ModEntry>{
        let res = self.get_compatible_versions(client, loader, version).await;
        if res.is_none(){
            eprintln!("No compatible versions found");
            return vec![];
        }
        self.get_specific_version();
        let deps = self.get_dependencies();
        self.get_download_params();
        deps

    }
}
impl Default for ModEntry {
    fn default() -> Self {
        Self { 
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
