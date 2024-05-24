use super::input::UserInput;

#[derive(Clone, Debug, Default)]
pub struct UserState {
    pub render_motd: bool,
    pub first_render: bool,
    pub input: UserInput,
}

impl UserState {
    pub fn new() -> Self {
        Self {
            render_motd: true,
            first_render: true,
            ..Default::default()
        }
    }
}
