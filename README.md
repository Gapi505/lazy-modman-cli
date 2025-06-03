# lazy-modman-cli

**lazy-modman-cli** is a minimalist Minecraft mod manager designed for users who reuse the same lightweight modpack across multiple game versions.  
Define your modpack once, and it will adapt it to any version—automatically.

---

### ✅ Best suited for:
- Simple quality-of-life modpacks.
- Packs without custom configs or tight version dependencies.
- Users tired of re-downloading the same mods for each version.
- Anyone who wants to avoid bloated, per-version Minecraft instances.

> *Example: A clean pack with Sodium, Iris Shaders, Continuity, etc.*

---

### ❌ Not recommended for:
- Heavy, gameplay-changing modpacks.
- Modpacks with complex or version-sensitive configurations.
- Packs like RLCraft or other highly curated setups.

---

### 🔧 How It Works

- Define your modpack once via a JSONC config file.
- Choose a Minecraft version when prompted.
- `lazy-modman-cli` fetches compatible mod versions from **Modrinth**.
- It automatically backs up your current `mods/` folder before installing.
- Downloads are cached locally to avoid unnecessary future requests.

---

### 🚀 Usage

1. **Install the tool**  
   Place the executable in your `.minecraft/` folder and run it once.  
   This will auto-create the `modpacks/` and `mods/` directories if they don’t exist.

2. **Create a modpack config**  
   - Place your JSONC config file into:  
     `.minecraft/modpacks/your-modpack-name.jsonc`
   - Use the included reference and example files for structure.

3. **Run the tool**  
   - **Linux/macOS:**
     ```bash
     ./lazy-modman-cli
     ```
   - **Windows:**  
     Just double-click the `.exe` from your `.minecraft/` folder.

4. **Follow the prompts**  
   - Select a modpack using autocomplete + tab completion.
   - Enter the Minecraft version you want to install mods for.

5. **Done**  
   - Your mods are downloaded, backed up, and replaced.
   - You're ready to launch Minecraft.

---

Minimal effort. Maximum compatibility.  
No instances. No bloat. No bullshit.
