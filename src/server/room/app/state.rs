use super::input::UserInput;

#[derive(Clone, Debug, Default)]
pub struct UserState {
    pub first_render: bool,
    pub input: UserInput,
}

impl UserState {
    pub fn new() -> Self {
        Self {
            first_render: true,
            ..Default::default()
        }
    }
}
