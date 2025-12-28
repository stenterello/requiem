use std::collections::HashMap;
use anyhow::Context;
use bevy::{asset::{LoadState, LoadedFolder}, prelude::*, time::Stopwatch};
use bevy_ui_widgets::{Activate, UiWidgetsPlugins};

use crate::{
    VisualNovelState, audio::controller::AudioResources, chat::{INFOTEXT_Z_INDEX_ACTIVE, INFOTEXT_Z_INDEX_INACTIVE, ui::{
        basic::{
            backplate_container, infotext_container, messagetext, namebox, nametext, textbox, top_section, vn_commands
        },
        history::history_panel
    }}, compiler::controller::{
        Controller, ControllerReadyMessage, ControllersSetStateMessage, SabiState, UiRoot
    }
};

const UI_ASSET_PATH: &str = "sabi/ui";
const UI_FONTS_PATH: &str = "sabi/fonts";

/* Messages */
#[derive(Message)]
pub(crate) struct CharacterSayMessage {
    pub name: Option<String>,
    pub message: String
}
#[derive(Message)]
pub(crate) struct InfoTextMessage {
    pub text: String
}
#[derive(Clone)]
pub(crate) enum UiChangeInnerMessage {
    Set {
        target_element: UiChangeTarget,
        target_property: String,
        image_mode: Option<UiImageMode>,
    },
    Unset { target_element: UiChangeTarget }
}
#[derive(Message)]
pub(crate) struct UiChangeMessage {
    pub command: UiChangeInnerMessage,
}
#[derive(Message)]
pub struct UiChangeDefaultFont(pub String);

/* States */
#[derive(States, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
pub(crate) enum ChatControllerState {
    #[default]
    Idle,
    Loading,
    Running,
}

#[derive(SubStates, Debug, Default, Clone, Copy, Hash, Eq, PartialEq)]
#[source(ChatControllerState = ChatControllerState::Running)]
pub(crate) enum ChatControllerSubState {
    #[default]
    Default,
    History
}

impl From<SabiState> for ChatControllerState {
    fn from(value: SabiState) -> Self {
        match value {
            SabiState::Idle => ChatControllerState::Idle,
            SabiState::WaitingForControllers => ChatControllerState::Loading,
            SabiState::Running => ChatControllerState::Running,
        }
    }
}

/* Components */
#[derive(Component, Default)]
pub(crate) struct GUIScrollText {
    pub message: String
}
#[derive(Component)]
pub(crate) struct VNContainer;
#[derive(Component)]
pub(crate) struct TextBoxBackground;
#[derive(Component)]
pub(crate) struct NameBoxBackground;
#[derive(Component)]
pub(crate) struct NameText;
#[derive(Component)]
pub(crate) struct MessageText;
#[derive(Component)]
pub(crate) struct InfoTextComponent;
#[derive(Component)]
pub(crate) struct InfoTextContainer;
#[derive(Component)]
pub(crate) struct VnCommands;
#[derive(Component)]
pub(crate) struct HistoryPanel;
#[derive(Component)]
pub(crate) struct HistoryScrollbar;
#[derive(Component)]
pub(crate) struct HistoryText;
#[derive(Component)]
pub(crate) struct UiAudioPlayer;
#[derive(Component)]
pub(crate) struct TypingAudioPlayer;

/* Resources */
#[derive(Resource)]
pub(crate) struct ChatScrollStopwatch(Stopwatch);
#[derive(Resource)]
struct HandleToUiFolder(Handle<LoadedFolder>);
#[derive(Resource)]
struct HandleToFontsFolder(Handle<LoadedFolder>);
#[derive(Resource)]
struct UiImages(HashMap<String, Handle<Image>>);
#[derive(Resource)]
pub(crate) struct CurrentTextBoxBackground(pub ImageNode);
#[derive(Resource, Default)]
pub(crate) struct FontRegistry(pub HashMap<String, Handle<Font>>);
#[derive(Resource)]
pub(crate) struct CurrentFont(pub Handle<Font>);
#[derive(Resource)]
pub(crate) struct DefaultFont(pub Handle<Font>);
#[derive(Resource, Default)]
pub(crate) struct UiFolderLoaded(pub bool);
#[derive(Resource, Default)]
pub(crate) struct FontsFolderLoaded(pub bool);
#[derive(Resource, Default)]
pub(crate) struct UiSounds(pub Option<Handle<AudioSource>>);
#[derive(Resource, Default)]
pub(crate) struct TypingSound(pub Option<Handle<AudioSource>>);

/* Custom types */
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum UiChangeTarget {
    TextBoxBackground,
    NameBoxBackground,
    Font,
    UiSounds,
    TypingSound,
}
impl TryFrom<&str> for UiChangeTarget {
    type Error = std::io::Error;
    
    fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
        match value {
            "textbox"      => Ok(UiChangeTarget::TextBoxBackground),
            "namebox"      => Ok(UiChangeTarget::NameBoxBackground),
            "font"         => Ok(UiChangeTarget::Font),
            "ui sfx"       => Ok(UiChangeTarget::UiSounds),
            "typing sound" => Ok(UiChangeTarget::TypingSound),
            other => Err(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!("Unexpected position: {:?}", other),
            ))
        }
    }
}
#[derive(Debug, Clone, Default)]
pub(crate) enum UiImageMode {
    Sliced,
    #[default]
    Auto
}
#[derive(Hash, Eq, PartialEq, Component, Clone, Debug)]
pub(crate) enum UiButtons {
    OpenHistory,
    ExitHistory,
    Rewind,
    TextBox,
    InfoText,
}

pub(crate) struct ChatController;
impl Plugin for ChatController {
    fn build(&self, app: &mut App){
        app.insert_resource(ChatScrollStopwatch(Stopwatch::new()))
            .insert_resource(UiFolderLoaded::default())
            .insert_resource(FontsFolderLoaded::default())
            .insert_resource(UiSounds::default())
            .insert_resource(TypingSound::default())
            .init_state::<ChatControllerState>()
            .init_state::<ChatControllerSubState>()
            .add_systems(OnEnter(ChatControllerState::Loading), import_assets)
            .add_systems(Update, setup.run_if(in_state(ChatControllerState::Loading)))
            .add_message::<CharacterSayMessage>()
            .add_message::<InfoTextMessage>()
            .add_message::<UiChangeMessage>()
            .add_message::<UiChangeDefaultFont>()
            .add_plugins(UiWidgetsPlugins)
            .add_systems(Update, wait_trigger)
            .add_systems(OnEnter(ChatControllerState::Running), spawn_chatbox)
            .add_systems(Update, (update_chatbox, update_infotext, update_ui).run_if(in_state(ChatControllerState::Running)))
            .add_systems(OnExit(ChatControllerState::Running), clean_resources)
            .add_observer(button_clicked_history_state)
            .add_observer(button_clicked_default_state);
    }
}
fn clean_resources(
    mut ui_loaded_folder: ResMut<UiFolderLoaded>,
    mut fonts_loaded_folder: ResMut<FontsFolderLoaded>,
) {
    ui_loaded_folder.0 = false;
    fonts_loaded_folder.0 = false;
}
fn button_clicked_history_state(
    trigger: On<Activate>,
    mut commands: Commands,
    q_buttons: Query<(Entity, &UiButtons)>,
    current_sub_state: Res<State<ChatControllerSubState>>,
    mut sub_state: ResMut<NextState<ChatControllerSubState>>,
    history_panel: Single<Entity, With<HistoryPanel>>,
    ui_sounds: Res<UiSounds>,
    ui_audio_player: Query<Entity, With<UiAudioPlayer>>,
) -> Result<(), BevyError> {

    if *current_sub_state != ChatControllerSubState::History {
        return Ok(())
    }

    let entity = q_buttons.get(trigger.entity).context("Clicked Entity does not have UiButtons declared")?;
    let clicked = match entity.1 {
        UiButtons::ExitHistory => {
            warn!("Exit history clicked");
            commands.entity(*history_panel).despawn();
            sub_state.set(ChatControllerSubState::Default);
            true
        },
        _ => { false }
    };
    
    if clicked {
        if let Some(sound) = &ui_sounds.0 {
            if !ui_audio_player.is_empty() {
                let entity = ui_audio_player.single().context("Unable to get ui audio player")?;
                commands.entity(entity).despawn();
            }
            commands.spawn((
                AudioPlayer::new(sound.clone()),
                UiAudioPlayer
            ));
        }
    }
    Ok(())
}
fn button_clicked_default_state<'a>(
    trigger: On<Activate>,
    mut commands: Commands,
    mut q_visibilities: Query<(
        &mut Visibility,
        Option<&VNContainer>,
        Option<&NameBoxBackground>,
        Option<&InfoTextComponent>,
    )>,
    scroll_stopwatch: ResMut<ChatScrollStopwatch>,
    mut message_text: Single<(&mut GUIScrollText, &mut Text), (With<MessageText>, Without<NameText>, Without<InfoTextComponent>)>,
    mut info_text: Single<(&mut GUIScrollText, &mut Text), (With<InfoTextComponent>, Without<NameText>, Without<MessageText>, Without<VNContainer>)>,
    info_text_container_zidx: Single<&mut ZIndex, (With<InfoTextContainer>, Without<VNContainer>)>,
    mut game_state: ResMut<VisualNovelState>,
    ui_root: Single<Entity, With<UiRoot>>,
    q_buttons: Query<(Entity, &UiButtons)>,
    current_plate: Res<CurrentTextBoxBackground>,
    current_font: Res<'a, CurrentFont>,
    current_sub_state: Res<State<ChatControllerSubState>>,
    ui_sounds: Res<UiSounds>,
    ui_audio_player: Query<Entity, With<UiAudioPlayer>>,
    q_typing_player: Query<&mut AudioSink, With<TypingAudioPlayer>>,
    mut sub_state: ResMut<NextState<ChatControllerSubState>>,
) -> Result<(), BevyError> {

    if *current_sub_state != ChatControllerSubState::Default {
        return Ok(())
    }
    
    let mut vn_vis = None;
    let mut name_vis = None;
    let mut info_vis = None;
    for (vis, is_vn, is_namebox, is_info) in q_visibilities.iter_mut() {
        if is_vn.is_some() { vn_vis = Some(vis); }
        else if is_namebox.is_some() { name_vis = Some(vis); }
        else if is_info.is_some() { info_vis = Some(vis); }
    }

    let entity = q_buttons.get(trigger.entity)
        .context("Clicked Entity does not have UiButtons declared")?;
    let clicked = match entity.1 {
        UiButtons::OpenHistory => {
            warn!("Open history clicked");
            let history_panel_id = commands.spawn(history_panel(current_plate, &game_state, current_font.0.clone())?).id();
            commands.entity(*ui_root).add_child(history_panel_id);
            sub_state.set(ChatControllerSubState::History);
            true
        },
        UiButtons::Rewind => {
            warn!("Rewind button clicked!");
            if let Ok(()) = game_state.set_rewind() {
                *info_text.0 = GUIScrollText::default();
                *message_text.0 = GUIScrollText::default();
            }
            true
        },
        UiButtons::TextBox => {
            warn!("Textbox history clicked");
            match (&mut vn_vis, &mut name_vis) {
                (Some(vn_v), Some(name_v)) => textbox_clicked(vn_v, name_v, scroll_stopwatch, message_text, &q_typing_player, game_state)?,
                _ => { return Err(anyhow::anyhow!("Error on elements visibility!").into()) }
            }
            false
        },
        UiButtons::InfoText => {
            warn!("Infotext container clicked");
            if let Some(i_vis) = &mut info_vis {
                infotext_clicked(scroll_stopwatch, info_text, i_vis, info_text_container_zidx, game_state);
            }
            false
        }
        _ => { false }
    };
    
    if clicked {
        if let Some(sound) = &ui_sounds.0 {
            if !ui_audio_player.is_empty() {
                let entity = ui_audio_player.single().context("Unable to get ui audio player")?;
                commands.entity(entity).despawn();
            }
            commands.spawn((
                AudioPlayer::new(sound.clone()),
                UiAudioPlayer
            ));
        }
    }

    Ok(())
}
fn infotext_clicked(
    mut scroll_stopwatch: ResMut<ChatScrollStopwatch>,
    info_text_data: Single<(&mut GUIScrollText, &mut Text), (With<InfoTextComponent>, Without<NameText>, Without<MessageText>, Without<VNContainer>)>,
    info_visibility: &mut Visibility,
    mut container_zidx: Single<&mut ZIndex, (With<InfoTextContainer>, Without<VNContainer>)>,
    mut game_state: ResMut<VisualNovelState>,
) {
    let length: u32 = (scroll_stopwatch.0.elapsed_secs() * 25.) as u32;
    if length < info_text_data.0.message.len() as u32 {
        // Skip message scrolling
        scroll_stopwatch.0.set_elapsed(std::time::Duration::from_secs_f32(100000000.));
        return;
    }
    println!("[ Infotext finished ]");

    // Allow transitions to be run again
    game_state.blocking = false;
    *info_visibility = Visibility::Hidden;
    **container_zidx = ZIndex(INFOTEXT_Z_INDEX_INACTIVE);
}
fn textbox_clicked(
    vncontainer_visibility: &mut Visibility,
    namebox_visibility: &mut Visibility,
    mut scroll_stopwatch: ResMut<ChatScrollStopwatch>,
    message_text: Single<(&mut GUIScrollText, &mut Text), (With<MessageText>, Without<NameText>, Without<InfoTextComponent>)>,
    q_typing_player: &Query<&mut AudioSink, With<TypingAudioPlayer>>,
    mut game_state: ResMut<VisualNovelState>,
) -> Result<(), BevyError> {
    let length: u32 = (scroll_stopwatch.0.elapsed_secs() * 50.) as u32;
    if length < message_text.0.message.len() as u32 {
        // Skip message scrolling
        scroll_stopwatch.0.set_elapsed(std::time::Duration::from_secs_f32(100000000.));
        if !q_typing_player.is_empty() {
            let player = q_typing_player.single().context("Unable to get typing player")?;
            player.stop();
        }
        return Ok(());
    }
    println!("[ Player finished message ]");

    // Hide textbox parent object
    *vncontainer_visibility = Visibility::Hidden;
    *namebox_visibility = Visibility::Hidden;

    // Allow transitions to be run again
    game_state.blocking = false;
    Ok(())
}
fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    loaded_folders: Res<Assets<LoadedFolder>>,
    ui_folder_handle: Res<HandleToUiFolder>,
    fonts_folder_handle: Res<HandleToFontsFolder>,
    mut ui_loaded_folder: ResMut<UiFolderLoaded>,
    mut fonts_loaded_folder: ResMut<FontsFolderLoaded>,
    mut controller_state: ResMut<NextState<ChatControllerState>>,
    mut msg_writer: MessageWriter<ControllerReadyMessage>,
) -> Result<(), BevyError> {
    
    // ui folder
    if !ui_loaded_folder.0 {
        if let Some(state) = asset_server.get_load_state(ui_folder_handle.0.id()) {
            let mut gui_sprites = HashMap::<String, Handle<Image>>::new();
            match state {
                LoadState::Loaded => {
                    if let Some(loaded_folder) = loaded_folders.get(ui_folder_handle.0.id()) {
                        for handle in &loaded_folder.handles {
                            let path = handle.path()
                                .context("Error retrieving gui path")?;
                            let filename = path.path().file_stem()
                                .context("GUI file has no name")?
                                .to_string_lossy()
                                .to_string();
                            gui_sprites.insert(filename, handle.clone().typed());
                            ui_loaded_folder.0 = true;
                        }
                    } else {
                        return Err(anyhow::anyhow!("Could not find chat loaded folder!").into());
                    }

                    commands.insert_resource(UiImages(gui_sprites));
                },
                LoadState::Failed(e) => {
                    return Err(anyhow::anyhow!("Error loading GUI assets: {}", e.to_string()).into());
                }
                _ => {}
            }
        }
    }
    
    // fonts folder
    if !fonts_loaded_folder.0 {
        if let Some(state) = asset_server.get_load_state(fonts_folder_handle.0.id()) {
            let mut fonts = HashMap::<String, Handle<Font>>::new();
            match state {
                LoadState::Loaded => {
                    if let Some(loaded_folder) = loaded_folders.get(fonts_folder_handle.0.id()) {
                        for handle in &loaded_folder.handles {
                            let path = handle.path()
                                .context("Error retrieving gui path")?;
                            let filename = path.path().file_stem()
                                .context("GUI file has no name")?
                                .to_string_lossy()
                                .to_string();
                            fonts.insert(filename, handle.clone().typed());
                            fonts_loaded_folder.0 = true;
                        }
                    } else {
                        return Err(anyhow::anyhow!("Could not find chat loaded folder!").into());
                    }

                    let default_handle = fonts.get("ALLER").context("Default font ALLER is not present")?.clone();
                    commands.insert_resource(DefaultFont(default_handle.clone()));
                    commands.insert_resource(CurrentFont(default_handle));
                    commands.insert_resource(FontRegistry(fonts));
                },
                LoadState::Failed(e) => {
                    return Err(anyhow::anyhow!("Error loading GUI assets: {}", e.to_string()).into());
                }
                _ => {}
            }
        }
    }
    
    if ui_loaded_folder.0 && fonts_loaded_folder.0 {
        controller_state.set(ChatControllerState::Idle);
        msg_writer.write(ControllerReadyMessage(Controller::Chat));
        info!("chat controller ready");
    }
    Ok(())
}
fn import_assets(mut commands: Commands, asset_server: Res<AssetServer> ){
    let loaded_folder_ui = asset_server.load_folder(UI_ASSET_PATH);
    let loaded_folder_fonts = asset_server.load_folder(UI_FONTS_PATH);
    commands.insert_resource(HandleToUiFolder(loaded_folder_ui));
    commands.insert_resource(HandleToFontsFolder(loaded_folder_fonts));
}
fn spawn_chatbox(
    mut commands: Commands,
    ui_root: Single<Entity, With<UiRoot>>,
    current_font: Res<CurrentFont>,
) -> Result<(), BevyError> {
    // Spawn Backplate + Nameplate
    // Container
    let container = commands.spawn(backplate_container()).id();
    commands.entity(ui_root.entity()).add_child(container);

    // Top section: Nameplate flex container
    let top_section = commands.spawn(top_section()).id();
    commands.entity(container).add_child(top_section);

    // Namebox Node
    let namebox = commands.spawn(namebox()).id();
    commands.entity(top_section).add_child(namebox);

    // NameText
    let nametext = commands.spawn(nametext(current_font.0.clone())).id();
    commands.entity(namebox).add_child(nametext);

    // Backplate Node
    let textbox_bg = commands.spawn(textbox()).id();
    commands.entity(container).add_child(textbox_bg);

    // MessageText
    let messagetext = commands.spawn(messagetext(current_font.0.clone())).id();
    commands.entity(textbox_bg).add_child(messagetext);

    // VN commands
    let vn_commands = commands.spawn(vn_commands()?).id();
    commands.entity(textbox_bg).add_child(vn_commands);

    // InfoText
    let infotext_container = commands.spawn(infotext_container(current_font.0.clone())).id();
    commands.entity(ui_root.entity()).add_child(infotext_container);
    
    Ok(())
}
fn update_chatbox(
    mut commands: Commands,
    mut event_message: MessageReader<CharacterSayMessage>,
    vncontainer_visibility: Single<&mut Visibility, (With<VNContainer>, Without<NameBoxBackground>)>,
    mut namebox_visibility: Single<&mut Visibility, (With<NameBoxBackground>, Without<VNContainer>)>,
    mut name_text: Single<&mut Text, (With<NameText>, Without<MessageText>)>,
    mut message_text: Single<(&mut GUIScrollText, &mut Text), (With<MessageText>, Without<NameText>)>,
    mut scroll_stopwatch: ResMut<ChatScrollStopwatch>,
    mut game_state: ResMut<VisualNovelState>,
    typing_sound: Res<TypingSound>,
    q_typing_player: Query<&AudioSink, With<TypingAudioPlayer>>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    // Tick clock
    let to_tick = if time.delta_secs() > 1. { std::time::Duration::from_secs_f32(0.) } else { time.delta() };
    scroll_stopwatch.0.tick(to_tick);
    let mut vncontainer_visibility = vncontainer_visibility.into_inner();

    /* STANDARD SAY EVENTS INITIALIZATION [Transition::Say] */
    for ev in event_message.read() {
        game_state.blocking = true;
        // Make the visual novel ui container visible
        *vncontainer_visibility = Visibility::Visible;
        // Reset the scrolling timer
        scroll_stopwatch.0.set_elapsed(std::time::Duration::from_secs_f32(0.));
        // Update the name
        if let Some(name) = &ev.name {
            let result = if name == "[_PLAYERNAME_]" { game_state.playername.clone() } else { name.clone() };
            name_text.0 = result;
            **namebox_visibility = Visibility::Visible;
        } else {
            **namebox_visibility = Visibility::Hidden;
        };
        
        info!("character: {:?}, dialogue {}", ev.name, ev.message);
        message_text.0.message = ev.message.clone();
        if let Some(sound) = &typing_sound.0 {
            if q_typing_player.is_empty() {
                commands.spawn((
                    AudioPlayer::new(sound.clone()),
                    TypingAudioPlayer
                ));
            } else {
                let player = q_typing_player.single().context("Unable to retrieve Typing audio player")?;
                player.play();
            }
        }
    }

    // If vn container is hidden, ignore the next section dedicated to updating it
    if *vncontainer_visibility == Visibility::Hidden {
        return Ok(());
    }

    // Take the original string from the message object
    let mut original_string: String = message_text.0.message.clone();

    // Get the section of the string according to the elapsed time
    let length: usize = (scroll_stopwatch.0.elapsed_secs() * 50.) as usize;
    
    if length >= original_string.len() {
        if let Some(_) = &typing_sound.0 {
            if !q_typing_player.is_empty() {
                let player = q_typing_player.single().context("Unable to retrieve Typing audio player")?;
                player.pause();
            }
        }
    }

    // Return the section and apply it to the text object
    original_string.truncate(length);
    message_text.1.0 = original_string;

    Ok(())
}
fn update_infotext(
    mut event_message: MessageReader<InfoTextMessage>,
    mut info_text: Single<(&mut GUIScrollText, &mut Text, &mut Visibility), With<InfoTextComponent>>,
    mut info_text_container_zidx: Single<&mut ZIndex, With<InfoTextContainer>>,
    mut scroll_stopwatch: ResMut<ChatScrollStopwatch>,
    mut game_state: ResMut<VisualNovelState>,
    time: Res<Time>,
) -> Result<(), BevyError> {
    // Tick clock
    let to_tick = if time.delta_secs() > 1. { std::time::Duration::from_secs_f32(0.) } else { time.delta() };
    scroll_stopwatch.0.tick(to_tick);

    /* STANDARD SAY EVENTS INITIALIZATION [Transition::Say] */
    for ev in event_message.read() {
        game_state.blocking = true;
        // Reset the scrolling timer
        scroll_stopwatch.0.set_elapsed(std::time::Duration::from_secs_f32(0.));
        // Update the name
        println!("INFOTEXT {}", ev.text);
        info_text.0.message = ev.text.clone();
        *info_text.2 = Visibility::Visible;
        **info_text_container_zidx = ZIndex(INFOTEXT_Z_INDEX_ACTIVE);
    }

    // Take the original string from the message object
    let mut original_string: String = info_text.0.message.clone();

    // Get the section of the string according to the elapsed time
    let length: u32 = (scroll_stopwatch.0.elapsed_secs() * 25.) as u32;

    // Return the section and apply it to the text object
    original_string.truncate(length as usize);
    info_text.1.0 = original_string;
    
    Ok(())
}
fn wait_trigger(
    mut msg_reader: MessageReader<ControllersSetStateMessage>,
    mut controller_state: ResMut<NextState<ChatControllerState>>,
) {
    for msg in msg_reader.read() {
        controller_state.set(msg.0.into());
    }
}
fn update_ui(
    mut commands: Commands,
    mut change_messages: MessageReader<UiChangeMessage>,
    mut change_default_font: MessageReader<UiChangeDefaultFont>,
    mut q_image_node: Query<
        (&mut ImageNode, Has<TextBoxBackground>, Has<NameBoxBackground>),
        Or<(With<TextBoxBackground>, With<NameBoxBackground>)>
    >,
    mut current_font: ResMut<CurrentFont>,
    mut default_font: ResMut<DefaultFont>,
    font_registry: Res<FontRegistry>,
    audios: Res<AudioResources>,
    mut ui_sounds: ResMut<UiSounds>,
    mut typing_sound: ResMut<TypingSound>,
    mut q_fonts: Query<&mut TextFont>,
    concrete_images: Res<Assets<Image>>,
    gui_images: Res<UiImages>,
) -> Result<(), BevyError> {
    
    for ev in change_default_font.read() {
        let new_font = font_registry.0.get(&ev.0).context(format!("Could not find font {}", ev.0))?;
        default_font.0 = new_font.clone();
    }
    
    for ev in change_messages.read() {
        match ev.command.clone() {
            UiChangeInnerMessage::Set { target_element, target_property, image_mode } => {
                match target_element {
                    UiChangeTarget::TextBoxBackground | UiChangeTarget::NameBoxBackground => {
                        let image = gui_images.0.get(&target_property)
                            .context(format!("UI asset '{}' does not exist", target_property))?;
                        let mut target = if target_element == UiChangeTarget::TextBoxBackground {
                            q_image_node.iter_mut().find(|q| q.1 == true)
                                .context("Unable to find textbox")?.0
                        } else {
                            q_image_node.iter_mut().find(|q| q.2 == true)
                                .context("Unable to find textbox")?.0
                        };
                        target.image = image.clone();
                        target.image_mode = match image_mode {
                            Some(UiImageMode::Sliced) => {
                                let concrete_image = concrete_images.get(image).context("Could not find image")?;
                                let concrete_image_size = concrete_image.texture_descriptor.size;
                                let slice_cuts = BorderRect {
                                    top: concrete_image_size.height as f32 / 5.,
                                    bottom: concrete_image_size.height as f32 / 5.,
                                    left: concrete_image_size.width as f32 / 5.,
                                    right: concrete_image_size.width as f32 / 5.
                                };
                                NodeImageMode::Sliced(TextureSlicer {
                                    border: slice_cuts,
                                    center_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
                                    sides_scale_mode: SliceScaleMode::Tile { stretch_value: 1. },
                                    ..default()
                                })
                            },
                            Some(UiImageMode::Auto) => NodeImageMode::Auto,
                            None => { return Err(anyhow::anyhow!("Ui Image Mode missing!").into()) }
                        };
                        if target_element == UiChangeTarget::TextBoxBackground {
                            commands.insert_resource(CurrentTextBoxBackground(target.clone()));
                        }
                    },
                    UiChangeTarget::Font => {
                        current_font.0 = font_registry.0.get(&target_property).context("Target font {target_property} not found in registry")?.clone();
                        for mut font in &mut q_fonts {
                            font.font = current_font.0.clone();
                        }
                    },
                    UiChangeTarget::UiSounds => {
                        let concrete_sound = audios.category("ui")?.get(&target_property).context(format!("Unable to find {target_property} sound"))?;
                        ui_sounds.0 = Some(concrete_sound.clone());
                    },
                    UiChangeTarget::TypingSound => {
                        let concrete_sound = audios.category("ui")?.get(&target_property).context(format!("Unable to find {target_property} sound"))?;
                        typing_sound.0 = Some(concrete_sound.clone());
                    }
                }
            },
            UiChangeInnerMessage::Unset { target_element } => {
                match target_element {
                    UiChangeTarget::UiSounds => ui_sounds.0 = None,
                    UiChangeTarget::TypingSound => typing_sound.0 = None,
                    UiChangeTarget::Font => {
                        for mut font in &mut q_fonts {
                            font.font = default_font.0.clone();
                        }
                    },
                    _ => { }
                }
            }
        }
    }

    Ok(())
}
