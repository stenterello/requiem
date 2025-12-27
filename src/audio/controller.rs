use std::collections::HashMap;

use anyhow::Context;
use bevy::{asset::{LoadState, LoadedFolder}, prelude::*};
use bevy_audio::Volume;

use crate::compiler::{controller::{Controller, ControllerReadyMessage, ControllersSetStateMessage, SabiState}};


const AUDIO_ASSET_PATH: &str = "sabi/audio";

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
enum AudioControllerState {
    /// During Idle state, [AudioController] waits for a [ControllersSetStateMessage]
    #[default]
    Idle,
    /// During Loading state, [AudioController] loads and wait for assets folder to be completely loaded
    Loading,
    /// In Running state [AudioController] handles AudioChangeMessage
    Running,
}

impl From<SabiState> for AudioControllerState {
    fn from(value: SabiState) -> Self {
        match value {
            SabiState::Idle => AudioControllerState::Idle,
            SabiState::WaitingForControllers => AudioControllerState::Loading,
            SabiState::Running => AudioControllerState::Running,
        }
    }
}

/* Components */
#[derive(Component)]
pub(crate) struct MusicAudio;
#[derive(Component)]
pub(crate) struct SfxAudio;
#[derive(Component)]
pub(crate) struct AudioSourceId(pub String);

/* Resources */
#[derive(Resource)]
pub(crate) struct HandleToAudioFolder(pub Handle<LoadedFolder>);
#[derive(Resource, Debug)]
pub(crate) struct AudioResources {
    music: HashMap<String, Handle<AudioSource>>,
    sfx: HashMap<String, Handle<AudioSource>>,
    ui: HashMap<String, Handle<AudioSource>>,
}
impl AudioResources {
    pub(crate) fn category(&self, category: &str) -> Result<&HashMap<String, Handle<AudioSource>>, BevyError> {
        match category {
            "music" => Ok(&self.music),
            "sfx"   => Ok(&self.sfx),
            "ui"    => Ok(&self.ui),
            other   => { return Err(anyhow::anyhow!("Unexpected audio category {:?}", other).into()); }
        }
    }
}

/* Messages */
#[derive(Message, Debug)]
pub(crate) struct AudioChangeMessage {
    pub command: AudioCommand,
    pub category: String,
    pub audio: String,
    pub volume: f32,
}

/* Custom Types */
#[derive(Debug, Clone)]
pub(crate) enum AudioCommand {
    Start,
    Stop,
    Pause,
    Unpause,
}

impl TryFrom<&str> for AudioCommand {
    type Error = std::io::Error;
    
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "start"   => Ok(AudioCommand::Start),
            "stop"    => Ok(AudioCommand::Stop),
            "pause"   => Ok(AudioCommand::Pause),
            "unpause" => Ok(AudioCommand::Unpause),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unexpected audio_command: {:?}", other),
            ))
        }
    }
}

pub(crate) struct AudioController;
impl Plugin for AudioController {
    fn build(&self, app: &mut App) {
        app.init_state::<AudioControllerState>()
            .add_message::<AudioChangeMessage>()
            .add_systems(Update, check_state_change)
            .add_systems(OnEnter(AudioControllerState::Loading), import_assets)
            .add_systems(Update, check_loading_state.run_if(in_state(AudioControllerState::Loading)))
            .add_systems(Update, (
                update_audio,
            ).run_if(in_state(AudioControllerState::Running)));
    }
}

fn check_loading_state(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    folder_handle: Res<HandleToAudioFolder>,
    mut msg_writer: MessageWriter<ControllerReadyMessage>,
    mut controller_state: ResMut<NextState<AudioControllerState>>,
) -> Result<(), BevyError> {
    
    if let Some(state) = asset_server.get_load_state(folder_handle.0.id()) {
        
        let mut music: HashMap<String, Handle<AudioSource>> = HashMap::new();
        let mut sfx: HashMap<String, Handle<AudioSource>> = HashMap::new();
        let mut ui: HashMap<String, Handle<AudioSource>> = HashMap::new();
        
        match state {
            LoadState::Loaded => {
                if let Some(loaded_folder) = loaded_folders.get(folder_handle.0.id()) {
                    for handle in &loaded_folder.handles {
                        let path = handle.path()
                            .context("Error retrieving audio path")?;
                        let audio: Handle<AudioSource> = asset_server.load(path);
                        let filename = path.path().file_stem()
                            .context("Audio file has no name")?
                            .to_string_lossy()
                            .to_string();
                        let category = match path.path()
                            .components().nth(2)
                            .context("Could not find audio category")?
                            .as_os_str().to_str()
                            .context("Error converting os str to str")? {
                            "music" => &mut music,
                            "sfx" => &mut sfx,
                            "ui" => &mut ui,
                            other => { return Err(anyhow::anyhow!("Invalid audio category {}", other).into()); }
                        };
                        
                        category.insert(filename, audio);
                    }
                    let resource = AudioResources {
                        music,
                        sfx,
                        ui
                    };
                    info!("Audio resource {resource:?}");
                    commands.insert_resource(resource);
                } else {
                    return Err(anyhow::anyhow!("Could not find audio loaded folder!").into());
                }
                controller_state.set(AudioControllerState::Idle);
                msg_writer.write(ControllerReadyMessage(Controller::Audio));
                info!("audio controller ready");
            },
            LoadState::Failed(e) => {
                return Err(anyhow::anyhow!("Error loading audio assets: {}", e.to_string()).into());
            }
            _ => {}
        }
    }
    
    Ok(())
}

fn import_assets(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
) {
    let loaded_folder = asset_server.load_folder(AUDIO_ASSET_PATH);
    commands.insert_resource(HandleToAudioFolder(loaded_folder));
}

fn check_state_change(
    mut msg_reader: MessageReader<ControllersSetStateMessage>,
    mut controller_state: ResMut<NextState<AudioControllerState>>,
) {
    for msg in msg_reader.read() {
        controller_state.set(msg.0.into());
    }
}

fn update_audio(
    mut commands: Commands,
    audios: Res<AudioResources>,
    mut q_sinks: ParamSet<(
        Query<(Entity, &mut AudioSink, &AudioSourceId), With<MusicAudio>>,
        Query<(Entity, &mut AudioSink, &AudioSourceId), With<SfxAudio>>,
    )>,
    mut msg_reader: MessageReader<AudioChangeMessage>,
) -> Result<(), BevyError> {
    
    for msg in msg_reader.read() {
        match msg.command {
            AudioCommand::Start => {
                let concrete_audio = audios.category(&msg.category)?
                    .get(&msg.audio).context(format!("Unable to find {} sound", msg.audio))?;
                
                match msg.category.as_str() {
                    "music" => {
                        if !q_sinks.p0().is_empty() {
                            let mut q_music_sink = q_sinks.p0();
                            let (entity, music_sink, _) = q_music_sink.single_mut()?;
                            music_sink.stop();
                            commands.entity(entity).despawn();
                        }
                    },
                    _ => {}
                }
                let audio_player = AudioPlayer::new(concrete_audio.to_owned());
                let playback_settings = PlaybackSettings {
                    volume: Volume::Linear(msg.volume),
                    ..default()
                };
                if msg.category.as_str() == "music" {
                    commands.spawn((
                        audio_player,
                        playback_settings,
                        AudioSourceId(msg.audio.clone()),
                        MusicAudio
                    ));
                } else if msg.category.as_str() == "sfx" {
                    commands.spawn((
                        audio_player,
                        playback_settings,
                        AudioSourceId(msg.audio.clone()),
                        SfxAudio
                    ));
                }
            },
            AudioCommand::Pause => {
                info!("PAUSE COMMAND {msg:?}");
                match msg.category.as_str() {
                    "music" => {
                        if !q_sinks.p0().is_empty() {
                            let mut q_music_sink = q_sinks.p0();
                            let (_, music_sink, _) = q_music_sink.single_mut()?;
                            music_sink.pause();
                        }
                    },
                    "sfx" => {
                        if !q_sinks.p1().is_empty() {
                            let q_sfx_sink = q_sinks.p1();
                            let (_, sfx_sink, _) = q_sfx_sink.iter().find(|(_, _, id)| id.0 == msg.audio)
                                .context(format!("Audio {} not found in World", msg.audio))?;
                            sfx_sink.pause();
                        }
                    },
                    _ => { return Err(anyhow::anyhow!("Forbidden category {}", msg.category).into()); }
                }
            },
            AudioCommand::Unpause => {
                match msg.category.as_str() {
                    "music" => {
                        if !q_sinks.p0().is_empty() {
                            let mut q_music_sink = q_sinks.p0();
                            let (_, music_sink, _) = q_music_sink.single_mut()?;
                            music_sink.play();
                        }
                    },
                    "sfx" => {
                        if !q_sinks.p1().is_empty() {
                            let q_sfx_sink = q_sinks.p1();
                            let (_, sfx_sink, _) = q_sfx_sink.iter().find(|(_, _, id)| id.0 == msg.audio)
                                .context(format!("Audio {} not found in World", msg.audio))?;
                            sfx_sink.play();
                        }
                    },
                    _ => { return Err(anyhow::anyhow!("Forbidden category {}", msg.category).into()); }
                }
            },
            AudioCommand::Stop => {
                match msg.category.as_str() {
                    "music" => {
                        if !q_sinks.p0().is_empty() {
                            let mut q_music_sink = q_sinks.p0();
                            let (entity, music_sink, _) = q_music_sink.single_mut()?;
                            music_sink.stop();
                            commands.entity(entity).despawn();
                        }
                    },
                    "sfx" => {
                        if !q_sinks.p1().is_empty() {
                            let q_sfx_sink = q_sinks.p1();
                            let (entity, sfx_sink, _) = q_sfx_sink.iter().find(|(_, _, id)| id.0 == msg.audio)
                                .context(format!("Audio {} not found in World", msg.audio))?;
                            sfx_sink.stop();
                            commands.entity(entity).despawn();
                        }
                    },
                    _ => { return Err(anyhow::anyhow!("Forbidden category {}", msg.category).into()); }
                }
            },
        }
    }
    
    Ok(())
}