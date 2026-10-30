//! The logo never changes on its own: its state is computed once from config.

use std::rc::Rc;

use super::{LogoConfig, model};
use crate::modules::{Shared, ViewState};

pub struct LogoSource {
    state: Rc<Shared<ViewState>>,
}

impl LogoSource {
    pub fn start(config: &LogoConfig) -> Rc<Self> {
        Rc::new(Self {
            state: Shared::new(model::view_state(config)),
        })
    }

    pub fn state(&self) -> Rc<Shared<ViewState>> {
        self.state.clone()
    }

    pub fn reconfigure(&self, config: &LogoConfig) {
        self.state.set(model::view_state(config));
    }
}
