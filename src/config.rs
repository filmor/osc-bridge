use std::{fs, path::Path};
use thiserror::Error;

use mlua::{StdLib, prelude::*};

pub struct Config;


// Ideas: Load 

impl Config {
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let data = fs::read(path)?;
        
        let lua = Lua::new();
        lua.load_from_std_lib(StdLib::ALL_SAFE)?;
        
        let table = lua.create_table()?;
        lua.globals().set("bridge", table)?;

        let chunk = lua.load(&data)
            .set_name("config")?;
        
        chunk.exec()?;
        
        lua.load("print(bridge.options.test)").exec()?;
        
        Ok(Config)
    }
}


#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("Error in Lua code")]
    Lua(#[from] LuaError),

    #[error("Error reading file")]
    Io(#[from] std::io::Error)
}