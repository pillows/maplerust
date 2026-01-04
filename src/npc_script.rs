use crate::npc_dialog::{DialogType, DialogResponse};
use std::collections::HashMap;

/// NPC script state machine
pub struct NpcScriptEngine {
    active_npc: Option<i32>,  // NPC ID currently talking
    script_state: ScriptState,
    state_data: HashMap<String, String>,  // Script-specific data storage
}

#[derive(Clone, Debug)]
pub enum ScriptState {
    Idle,
    Waiting,              // Waiting for player response
    DialogPage(usize),    // Multi-page dialog, current page index
    SelectionWait,        // Waiting for selection
    StyleWait,            // Waiting for style selection
}

/// Commands returned by script engine
#[derive(Clone, Debug)]
pub enum NpcScriptCommand {
    None,
    ShowDialog {
        text: String,
        dialog_type: DialogType,
    },
    ShowSelection {
        text: String,
        options: Vec<String>,
    },
    ShowStyle {
        text: String,
        style_type: StyleType,  // Hair, Face, Skin
        available_styles: Vec<i32>,
    },
    GiveItem(i32, i32),  // item_id, quantity
    GiveMeso(i32),
    GiveExp(i32),
    TakeItem(i32, i32),
    Warp(i32),  // map_id
    Close,
}

#[derive(Clone, Copy, Debug)]
pub enum StyleType {
    Hair,
    Face,
    Skin,
}

impl NpcScriptEngine {
    pub fn new() -> Self {
        Self {
            active_npc: None,
            script_state: ScriptState::Idle,
            state_data: HashMap::new(),
        }
    }

    /// Start NPC interaction
    pub fn start_npc(&mut self, npc_id: i32) -> NpcScriptCommand {
        self.active_npc = Some(npc_id);
        self.script_state = ScriptState::Waiting;
        self.state_data.clear();

        // Route to appropriate script based on NPC ID
        match npc_id {
            1012100 => self.script_henesys_chief_start(),
            9000000 => self.script_test_npc_start(),
            9000001 => self.script_selection_test(),
            _ => self.script_default(),
        }
    }

    /// Handle player response and advance script
    pub fn handle_response(&mut self, response: DialogResponse) -> NpcScriptCommand {
        let npc_id = match self.active_npc {
            Some(id) => id,
            None => return NpcScriptCommand::None,
        };

        match (&self.script_state, response) {
            (ScriptState::Waiting, DialogResponse::Ok) => {
                // Simple OK - end dialog
                self.end_dialog()
            }
            (ScriptState::Waiting, DialogResponse::Next) => {
                // Advance to next page
                self.script_state = ScriptState::DialogPage(1);
                self.continue_script(npc_id, 1)
            }
            (ScriptState::DialogPage(page), DialogResponse::Next) => {
                // Continue multi-page dialog
                let next_page = page + 1;
                self.script_state = ScriptState::DialogPage(next_page);
                self.continue_script(npc_id, next_page)
            }
            (ScriptState::Waiting, DialogResponse::Yes) => {
                // Handle Yes response
                self.handle_yes(npc_id)
            }
            (ScriptState::Waiting, DialogResponse::No) => {
                // Handle No response
                self.end_dialog()
            }
            (ScriptState::SelectionWait, DialogResponse::Selection(idx)) => {
                // Handle selection
                self.handle_selection(npc_id, idx)
            }
            _ => NpcScriptCommand::None,
        }
    }

    fn end_dialog(&mut self) -> NpcScriptCommand {
        self.active_npc = None;
        self.script_state = ScriptState::Idle;
        self.state_data.clear();
        NpcScriptCommand::Close
    }

    fn continue_script(&self, npc_id: i32, page: usize) -> NpcScriptCommand {
        // Based on current NPC and state, return next command
        match npc_id {
            1012100 => self.script_henesys_chief_page(page),
            9000000 => self.script_test_npc_page(page),
            _ => NpcScriptCommand::Close,
        }
    }

    fn handle_yes(&mut self, npc_id: i32) -> NpcScriptCommand {
        match npc_id {
            1012100 => {
                // Henesys Chief - Give tour
                NpcScriptCommand::ShowDialog {
                    text: "Great! Let me give you a tour of Henesys. This is the town square where everyone gathers. To the north, you'll find shops and the job advancement center.".to_string(),
                    dialog_type: DialogType::Ok,
                }
            }
            _ => self.end_dialog(),
        }
    }

    fn handle_selection(&mut self, npc_id: i32, idx: usize) -> NpcScriptCommand {
        match npc_id {
            9000001 => {
                // Selection test NPC
                match idx {
                    0 => NpcScriptCommand::ShowDialog {
                        text: "You selected Option 1! That's a great choice.".to_string(),
                        dialog_type: DialogType::Ok,
                    },
                    1 => NpcScriptCommand::ShowDialog {
                        text: "You selected Option 2! Interesting decision.".to_string(),
                        dialog_type: DialogType::Ok,
                    },
                    2 => NpcScriptCommand::ShowDialog {
                        text: "You selected Option 3! The bold choice!".to_string(),
                        dialog_type: DialogType::Ok,
                    },
                    _ => self.end_dialog(),
                }
            }
            _ => self.end_dialog(),
        }
    }

    // Example scripts

    fn script_default(&self) -> NpcScriptCommand {
        NpcScriptCommand::ShowDialog {
            text: "Hello! How can I help you today?".to_string(),
            dialog_type: DialogType::Ok,
        }
    }

    fn script_henesys_chief_start(&self) -> NpcScriptCommand {
        NpcScriptCommand::ShowDialog {
            text: "Welcome to Henesys! Are you new here?".to_string(),
            dialog_type: DialogType::YesNo,
        }
    }

    fn script_henesys_chief_page(&self, page: usize) -> NpcScriptCommand {
        match page {
            1 => NpcScriptCommand::ShowDialog {
                text: "I'm the chief of Henesys village. We're a peaceful community of merchants and warriors.".to_string(),
                dialog_type: DialogType::Next,
            },
            2 => NpcScriptCommand::ShowDialog {
                text: "If you ever need help, feel free to ask any of the NPCs around town. Good luck on your journey!".to_string(),
                dialog_type: DialogType::Ok,
            },
            _ => NpcScriptCommand::Close,
        }
    }

    fn script_test_npc_start(&self) -> NpcScriptCommand {
        NpcScriptCommand::ShowDialog {
            text: "Hi! I'm a test NPC. Let me tell you a multi-page story...".to_string(),
            dialog_type: DialogType::Next,
        }
    }

    fn script_test_npc_page(&self, page: usize) -> NpcScriptCommand {
        match page {
            1 => NpcScriptCommand::ShowDialog {
                text: "Once upon a time, in the land of MapleStory, there lived a brave adventurer...".to_string(),
                dialog_type: DialogType::Next,
            },
            2 => NpcScriptCommand::ShowDialog {
                text: "This adventurer traveled far and wide, battling monsters and making friends.".to_string(),
                dialog_type: DialogType::Next,
            },
            3 => NpcScriptCommand::ShowDialog {
                text: "And they all lived happily ever after! The end.".to_string(),
                dialog_type: DialogType::Ok,
            },
            _ => NpcScriptCommand::Close,
        }
    }

    fn script_selection_test(&mut self) -> NpcScriptCommand {
        self.script_state = ScriptState::SelectionWait;
        NpcScriptCommand::ShowSelection {
            text: "Hello! Please choose one of the following options:".to_string(),
            options: vec![
                "Option 1 - Get a free potion".to_string(),
                "Option 2 - Warp to another map".to_string(),
                "Option 3 - Learn more about NPCs".to_string(),
            ],
        }
    }
}

impl Default for NpcScriptEngine {
    fn default() -> Self {
        Self::new()
    }
}
