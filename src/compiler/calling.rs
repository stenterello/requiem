use crate::audio::controller::AudioChangeMessage;
use crate::chat::controller::{InfoTextMessage, UiChangeTarget};
use crate::{BackgroundChangeMessage, CharacterSayMessage, UiChangeMessage, ActorChangeMessage, VisualNovelState};
use crate::compiler::ast::{CodeStatement, Dialogue, Evaluate, InfoText, StageCommand, Statement, TextItem};
use bevy::prelude::*;
use anyhow::{Context, Result};

/* Messages */
#[derive(Message)]
pub struct SceneChangeMessage {
    pub scene_id: String
}

#[derive(Message)]
pub struct ActChangeMessage {
    pub act_id: String
}

pub struct InvokeContext<'l, 'a, 'b, 'd, 'e, 'f, 'g, 'h, 'i, 'j> {
    pub game_state: &'l mut ResMut<'a, VisualNovelState>,
    pub character_say_message: &'l mut MessageWriter<'b, CharacterSayMessage>,
    pub background_change_message: &'l mut MessageWriter<'d, BackgroundChangeMessage>,
    pub gui_change_message: &'l mut MessageWriter<'e, UiChangeMessage>,
    pub scene_change_message: &'l mut MessageWriter<'f, SceneChangeMessage>,
    pub act_change_message: &'l mut MessageWriter<'g, ActChangeMessage>,
    pub actor_change_message: &'l mut MessageWriter<'h, ActorChangeMessage>,
    pub info_text_message: &'l mut MessageWriter<'i, InfoTextMessage>,
    pub audio_change_message: &'l mut MessageWriter<'j, AudioChangeMessage>,
}
pub trait Invoke {
    fn invoke ( &self, ctx: InvokeContext ) -> Result<()>;
}
impl Invoke for Dialogue {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        let dialogue = self.dialogue.evaluate_into_string()
            .context("...while evaluating Dialogue expression")?;
        info!("Invoking Dialogue::Say");

        ctx.character_say_message.write(CharacterSayMessage {
            name: self.character.to_owned(),
            message: dialogue
        });

        ctx.game_state.blocking = true;

        Ok(())
    }
}
impl Invoke for InfoText {
    fn invoke ( &self, ctx: InvokeContext ) -> Result<()> {
        let text = self.infotext.evaluate_into_string()
            .context("...while evaluating InfoText expression")?;
        info!("Invoking InfoText");
        
        
        // This is needed to prevent remaining stuck during a rewind process:
        // if the user goes backwards to a infotext statement, it will cause
        // the game_state to block and the vn commands to disappear, making it
        // impossible to go backwards beyond the infotext.
        if ctx.game_state.rewinding == 0 {
            ctx.info_text_message.write(InfoTextMessage {
                text,
            });
        
            ctx.game_state.blocking = true;
        }
        
        Ok(())
    }
}
impl Invoke for StageCommand {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        match self {
            StageCommand::BackgroundChange { operation } => {
                info!("Invoking StageCommand::BackgroundChange to {:?}", operation);
                ctx.background_change_message.write(BackgroundChangeMessage {
                    operation: operation.clone(),
                });
            },
            StageCommand::UiChange { ui_target, target_font, sprite_expr, image_mode, ui_sounds, typing_sound } => {
                let ui_target = ui_target.clone();
                let message = match ui_target {
                    UiChangeTarget::Font => {
                        let target_font = target_font.clone().context("Target font field empty")?;
                        let target_font_str = target_font.evaluate_into_string()?;
                        info!("Invoking StageCommand::UiChange font to {}", target_font_str);
                        UiChangeMessage {
                            ui_target,
                            target_font: Some(target_font_str),
                            sprite_id: None,
                            image_mode: None,
                            ui_sounds: None,
                            typing_sound: None,
                        }
                    },
                    UiChangeTarget::UiSounds => {
                        let target_sound = ui_sounds.clone().context("ui_sounds field empty")?;
                        let target_sound_str = target_sound.evaluate_into_string()?;
                        info!("Invoking StageCommand::UiChange ui sounds to {}", target_sound_str);
                        UiChangeMessage {
                            ui_target,
                            target_font: None,
                            sprite_id: None,
                            image_mode: None,
                            ui_sounds: Some(target_sound_str),
                            typing_sound: None,
                        }
                    },
                    UiChangeTarget::TypingSound => {
                        let target_sound = typing_sound.clone().context("typing field empty")?;
                        let target_sound_str = target_sound.evaluate_into_string()?;
                        info!("Invoking StageCommand::UiChange typing sound to {}", target_sound_str);
                        UiChangeMessage {
                            ui_target,
                            target_font: None,
                            sprite_id: None,
                            image_mode: None,
                            ui_sounds: None,
                            typing_sound: Some(target_sound_str),
                        }
                    },
                    _ => {
                        let sprite_expr = sprite_expr.clone().context("Sprite expr empty")?;
                        let sprite_id = sprite_expr.evaluate_into_string()
                            .context("...while evaluating UiChange sprite expression")?;
                        let image_mode = image_mode.clone();
                        info!("Invoking StageCommand::UiChange to {:?}'s {}", ui_target, sprite_id);
                        UiChangeMessage {
                            ui_target,
                            target_font: None,
                            sprite_id: Some(sprite_id),
                            image_mode,
                            ui_sounds: None,
                            typing_sound: None,
                        }
                    }
                };
                
                ctx.gui_change_message.write(message);
            },
            StageCommand::SceneChange { scene_expr } => {
                let scene_id = scene_expr.evaluate_into_string()
                    .context("...while evaluating SceneChange expression")?;
                
                info!("Invoking StageCommand::SceneChange to {}", scene_id);
                ctx.scene_change_message.write(SceneChangeMessage {
                    scene_id
                });
            },
            StageCommand::ActChange { act_expr } => {
                let act_id = act_expr.evaluate_into_string()
                    .context("...while evaluating ActChange expression")?;
                
                info!("Invoking StageCommand::ActChange to {}", act_id);
                ctx.act_change_message.write(ActChangeMessage {
                    act_id
                });
            },
            StageCommand::CharacterChange { character, operation } => {
                info!("Invoking StageCommand::CharacterChange to {} of type {:?}", character, operation);
                let message = ActorChangeMessage {
                    name: character.clone(),
                    operation: operation.clone()
                };
                ctx.actor_change_message.write(message);
            },
            StageCommand::AnimationChange { animation, operation } => {
                info!("Invoking StageCommand::AnimationChange to {} of type {:?}", animation, operation);
                let message = ActorChangeMessage {
                    name: animation.clone(),
                    operation: operation.clone()
                };
                ctx.actor_change_message.write(message);
            },
            StageCommand::AudioChange { command, category, audio, volume } => {
                info!("Invoking StageCommand::AudioChange command {:?} category {} audio {:?}", command, category, audio);
                let message = AudioChangeMessage { command: command.clone(), category: category.clone(), audio: audio.clone(), volume: volume.clone() };
                ctx.audio_change_message.write(message);
            }
        }
        
        Ok(())
    }
}
impl Invoke for CodeStatement {
    fn invoke( &self, _ctx: InvokeContext ) -> Result<()> {
        match self {
            CodeStatement::Log { exprs } => {
                let mut log_parts: Vec<String> = Vec::new();

                for expr in exprs {
                    let part = expr.evaluate_into_string()
                        .context("...while evaluating Log expression")?;
                    log_parts.push(part);
                }

                let log_message = log_parts.join(" ");
                println!("[ Log ] {}", log_message);

                Ok(())
            },
        }
    }
}
impl Invoke for Statement {
    fn invoke( &self, ctx: InvokeContext ) -> Result<()> {
        Ok(match self {
            Statement::TextItem(textitem) => {
                match textitem {
                    TextItem::Dialogue(dialogue) => dialogue.invoke(ctx)
                        .context("...while invoking Dialogue statement")?,
                    TextItem::InfoText(infotext) => infotext.invoke(ctx)
                        .context("...while invoking InfoText statement")?, 
                }
            }
            Statement::Stage(stage) => stage.invoke(ctx)
                .context("...while invoking StageCommand statement")?,
            Statement::Code(code) => code.invoke(ctx)
                .context("...while invoking Code statement")?,
        })
    }
}