# Sabi
**A Modern Visual Novel Game Engine**

Sabi is a cutting-edge visual novel engine built with Rust and Bevy, featuring dynamic character interactions and flexible scripting capabilities. Create immersive, responsive visual novels with rich character systems, dynamic backgrounds, and engaging dialogue.

![Rust](https://img.shields.io/badge/rust-%23000000.svg?style=for-the-badge&logo=rust&logoColor=white)
![Bevy](https://img.shields.io/badge/bevy-2C2D33?style=for-the-badge&logo=bevy&logoColor=white)

## ✨ Features

### 🎭 **Advanced Character System**
- **Dynamic Character Management**: JSON-based character definitions with customizable attributes
- **Emotion System**: Real-time emotion changes that affect character sprites and dialogue
- **Multi-Outfit Support**: Characters can switch between different outfits and emotional states
- **Character Descriptions**: Rich personality profiles for immersive storytelling

### 🎨 **Rich Visual Experience**
- **Dynamic Backgrounds**: Environment changes based on story progression
- **Character Sprites**: Emotion-based sprite switching with fade transitions
- **Custom GUI System**: Modular interface with themed textboxes and UI elements
- **Typing Animation**: Smooth text scrolling effects for immersive reading

### 📝 **Flexible Scripting Engine**
- **Custom Script Language**: Bash-like syntax for easy story creation
- **Scene Management**: Seamless transitions between story segments
- **Command System**: Rich set of commands for controlling game flow
- **Event-Driven Architecture**: Responsive system for handling user interactions

### 🔧 **Developer-Friendly**
- **Modular Plugin System**: Built on Bevy's ECS architecture
- **Hot-Reloadable Assets**: Dynamic loading of scripts, sprites, and configurations
- **Cross-Platform**: Runs on Windows, macOS, and Linux
- **Nix Integration**: Reproducible development environment with flake.nix

## 🚀 Quick Start

### Prerequisites
- Rust (latest stable)
- Git

### Installation

1. **Clone the repository:**
   ```bash
   git clone https://github.com/yourusername/sabi.git
   cd sabi
   ```

2. **Run the game:**
   ```bash
   cargo run
   ```

### Using Nix (Recommended)
```bash
nix develop  # Enter development shell
cargo run    # Build and run
```

## 📚 Script Language Reference

Sabi uses a custom scripting language with bash-like syntax for defining story flow:

### Basic Commands

```bash
# Character dialogue
say character=`Nayu` msg=`Hello, how are you today?`

# Player dialogue
psay msg=`I'm doing great, thanks for asking!`

# Set character emotion
set type=`emotion` character=`Nayu` emotion=`HAPPY`

# Change background
set type=`background` background=`main_classroom_day`

# Scene transitions
scene id=`scene2`

# Logging (development)
log msg=`Debug message here`
```

### Advanced Features

```bash
# GUI customization
set type=`GUI` id=`_textbox_background` sprite=`TEXTBOX_NAYU`

# End scene
end
```

## 🏗️ Architecture

Sabi is built on Bevy's Entity Component System (ECS) with distinct modules:

- **Compiler Module**: Parses script files and converts them to executable transitions
- **Character Module**: Manages character sprites, emotions, and properties
- **Chat Module**: Handles dialogue display and text animation
- **Background Module**: Controls scene backgrounds and environmental changes

## 🔧 Configuration

### Game Settings
Player name and other settings are currently configured in `src/main.rs`:

```rust
game_state.playername = String::from("YourName");
```

## 🤝 Contributing

We welcome contributions! Here are some areas where you can help:

- **UI/UX Improvements**: Enhanced text input, visual effects
- **Script Language Features**: New commands and functionality  
- **Performance Optimization**: Better asset loading and memory management
- **Cross-Platform Support**: Testing and fixes for different platforms

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## 📋 Roadmap

### Completed ✅
- [x] Character system with emotions and outfits
- [x] Custom scripting language
- [x] Scene management
- [x] Dynamic backgrounds
- [x] Text rendering and animation

### In Progress 🚧
- [ ] Enhanced text input system
- [ ] Visual transition effects
- [ ] Save/load system
- [ ] Audio integration

### Planned 📅
- [ ] Visual script editor
- [ ] Multiplayer support
- [ ] Mobile platform support
- [ ] Steam Workshop integration
- [ ] Advanced character interaction system

## 🙏 Acknowledgments

- Built with [Bevy Engine](https://bevyengine.org/)
- Development environment managed with [Nix](https://nixos.org/)
- Special thanks to the Rust and game development communities

## Assets credits

- Ui Panels (9 slice types) made by [BDragon1727](https://bdragon1727.itch.io/custom-border-and-panels-menu-all-part)
- Fire animation made by [Devkidd](https://devkidd.itch.io/pixel-fire-asset-pack)
- TypeWriter sound made by [kakaist](https://pixabay.com/users/kakaist-48093450/)
- Background music made by [D-wheat music](https://d-wheat-music.itch.io/free-background-music-for-visual-novels-vol3)
- Ui sounds made by [ConFuocoV](https://confuocov.itch.io/50-useme4free-digital-ui-sounds-sample-pack)
- Fire sound made by [SoundReality](https://pixabay.com/users/soundreality-31074404/)