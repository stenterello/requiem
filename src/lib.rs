mod background;
mod actor;
mod chat;
mod compiler;
mod loader;
mod audio;

use std::fmt::Debug;

use crate::audio::controller::AudioCommand;
use crate::audio::controller::AudioController;
use crate::background::*;
use crate::actor::controller::ActorConfig;
use crate::actor::controller::AnimationConfig;
use crate::actor::*;
use crate::chat::controller::UiChangeTarget;
use crate::chat::*;
use crate::compiler::ast::Evaluate;
use crate::compiler::ast::StageCommand;
use crate::compiler::ast::Statement;
use crate::compiler::ast::TextItem;
use crate::compiler::*;
use crate::loader::ActorJsonLoader;
use crate::loader::PestLoader;

use bevy::prelude::*;
use bevy::ecs::error::ErrorContext;

pub(crate) trait VariantKind {
    fn kind(&self) -> usize;
}

impl VariantKind for ast::Statement {
    fn kind(&self) -> usize {
        match self {
            Statement::TextItem(_) => 1,
            Statement::Stage(s) => {
                match s {
                    StageCommand::ActChange        { .. } => { 2 },
                    StageCommand::AnimationChange { operation, .. } => {
                        match operation {
                            ActorOperation::Spawn(_) |
                            ActorOperation::Despawn(_) => { 3 },
                            _ => { 4 }
                        }
                    },
                    StageCommand::AudioChange { command, category, .. } => {
                        match (command, category.as_str()) {
                            (AudioCommand::Start,   "music") |
                            (AudioCommand::Stop,    "music")   => { 5 },
                            (AudioCommand::Pause,   "music") |
                            (AudioCommand::Unpause, "music")   => { 6 },
                            (AudioCommand::Start,   "sfx") |
                            (AudioCommand::Stop,    "sfx")     => { 7 },
                            (AudioCommand::Pause,   "sfx") |
                            (AudioCommand::Unpause, "sfx")     => { 8 },
                            _                                  => { 9 },
                        }
                    },
                    StageCommand::BackgroundChange { .. } => { 10 },
                    StageCommand::CharacterChange  { .. } => { 11 },
                    StageCommand::SceneChange      { .. } => { 12 },
                    StageCommand::UiChange         { command } => {
                        match command {
                            ast::UiChangeCommand::Set { target_element, .. } => {
                                match target_element {
                                    UiChangeTarget::Font              => { 13 },
                                    UiChangeTarget::TextBoxBackground => { 14 },
                                    UiChangeTarget::NameBoxBackground => { 15 },
                                    UiChangeTarget::TypingSound       => { 16 },
                                    UiChangeTarget::UiSounds          => { 17 },
                                }
                            },
                            ast::UiChangeCommand::Unset { target_element } => {
                                match target_element {
                                    UiChangeTarget::Font              => { 13 },
                                    UiChangeTarget::TextBoxBackground => { 14 },
                                    UiChangeTarget::NameBoxBackground => { 15 },
                                    UiChangeTarget::TypingSound       => { 16 },
                                    UiChangeTarget::UiSounds          => { 17 },
                                }
                            }
                        }
                    },
                }
            }
            Statement::Code(_)     => 18,
        }
    }
}

#[derive(Default)]
pub(crate) struct Cursor<T> {
    data: Vec<T>,
    pos: i32,
}

impl Default for Cursor<ast::Statement> {
    fn default() -> Self {
        Cursor { data: Vec::default(), pos: -1 }
    }
}

impl<T> Cursor<T> {
    pub(crate) fn new(vec: Vec<T>) -> Self {
        Self {
            data: vec,
            pos: -1,
        }
    }
    
    pub(crate) fn next(&mut self) -> Option<T>
    where
        T: Clone
    {
        self.pos += 1;
        self.data.get(self.pos as usize).cloned()
    }

    pub(crate) fn prev(&mut self) -> Option<T>
    where
        T: Clone
    {
        info!("current pos {}", self.pos);
        if self.pos == 0 { return None; }
        self.pos -= 1;
        self.data.get(self.pos as usize).cloned()
    }

    pub(crate) fn find_previous(&self) -> Option<T>
    where
        T: Clone + VariantKind
    {
        info!("self.pos is {}", self.pos);
        if let Some(item) = self.data.get(self.pos as usize) {
            let current_kind = item.kind();
            info!("current kind {current_kind}");
            let mut idx: i32 = self.pos as i32 - 1;
            while idx >= 0 {
                if let Some(back_item) = self.data.get(idx as usize) {
                    info!("kind {}", back_item.kind());
                    if back_item.kind() == current_kind {
                        return Some(back_item.clone());
                    }
                } else { return None; }
                idx -= 1;
            }
            None
        } else {
            None
        }
    }
}

/// Resource containing main [Act] state and related runtime data for the Visual Novel.
/// Player-designated constants are passe by the [UserDefinedConstants] resource.
#[derive(Resource, Default)]
pub(crate) struct VisualNovelState {
    // Player-designated constants
    playername: String,

    pub act: Box<ast::Act>,
    pub scene: Box<ast::Scene>,
    pub statements: Cursor<ast::Statement>,
    blocking: bool,
    pub rewinding: usize,
    pub history: Vec<ast::Statement>,
}

impl VisualNovelState {
    pub fn set_rewind(&mut self) -> Result<(), BevyError> {
        if !self.text_before() {
            return Err(anyhow::anyhow!("No previous valid statement").into());
        }
        let search_slice = &self.history[..self.history.len() - 1];
        let last_d = search_slice.iter().rposition(|s| {
            matches!(s, Statement::TextItem(TextItem::Dialogue(_)))
        });
        let is_other_scene = search_slice.iter().rposition(|s| {
            matches!(s, Statement::Stage(StageCommand::SceneChange { .. })) ||
            matches!(s, Statement::Stage(StageCommand::ActChange { .. }))
        });
        // Notice: control over is_other_scene is needed since we don't handle 
        // rewinding system which goes back to a previous scene. 
        // With this control, we prevent stall.
        // Of course, it should be deleted as soon as we have the functionality
        match (last_d, is_other_scene) {
            (Some(index), Some(other_scene)) => {
                if index < other_scene {
                    return Err(anyhow::anyhow!("No previous valid statement").into());
                } else {
                    self.rewinding = self.history.len() - (index + 1);
                    self.blocking = false;
                    Ok(())
                }
            },
            (Some(index), None) => {
                self.rewinding = self.history.len() - (index + 1);
                self.blocking = false;
                Ok(())
            },
            (None, _) => {
                return Err(anyhow::anyhow!("No previous valid statement").into());
            }
        }
    }

    pub fn history_summary(&self) -> Result<Vec<String>> {
        let mut text: Vec<String> = Vec::new();

        for statement in &self.history {
            match statement {
                Statement::TextItem(t) => {
                    match t {
                        TextItem::Dialogue(d) => {
                            if let Some(c) = &d.character {
                                text.push(c.clone() + format!(": {}\n", d.dialogue.evaluate_into_string()?).as_str());
                            } else {
                                text.push(format!(": {}\n", d.dialogue.evaluate_into_string()?));
                            }
                        },
                        TextItem::InfoText(i) => {
                            text.push(i.infotext.evaluate_into_string()? + "\n");
                        }
                    }
                },
                Statement::Stage(StageCommand::SceneChange { scene_expr }) => {
                    text.push(format!("\nScene: {}\n", scene_expr.evaluate_into_string()?));
                },
                Statement::Stage(StageCommand::ActChange { act_expr }) => {
                    text.push(format!("\nAct: {}\n", act_expr.evaluate_into_string()?));
                },
                _ => {}
            }
        }

        Ok(text)
    }
    
    fn text_before(&self) -> bool {
        let search_slice = &self.history[..self.history.len() - 1];
        let last_d = search_slice.iter().rposition(|s| {
            matches!(s, Statement::TextItem(TextItem::Dialogue(_)))
        });
        last_d.is_some()
    }
}

#[derive(Resource, Default)]
pub struct UserDefinedConstants {
    pub playername: String,
}

fn sabi_error_handler ( err: BevyError, ctx: ErrorContext ) {
    panic!("Bevy error: {err:?}\nContext: {ctx:?}")
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ScriptId {
    pub chapter: String,
    pub act: String,
}

#[derive(Message)]
pub struct SabiStart(pub ScriptId);
#[derive(Message)]
pub struct SabiEnd;

pub struct SabiPlugin;
impl Plugin for SabiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserDefinedConstants>()
            .init_resource::<VisualNovelState>()
            .init_asset::<ActorConfig>()
            .init_asset::<CharacterConfig>()
            .init_asset::<AnimationConfig>()
            .init_asset_loader::<ActorJsonLoader>()
            .init_asset::<ast::Act>()
            .init_asset_loader::<PestLoader>()
            .set_error_handler(sabi_error_handler)
            .add_plugins((
                Compiler,
                BackgroundController,
                CharacterController,
                ChatController,
                AudioController,
            ));
    }
}
