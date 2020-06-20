use serde::Deserialize;
use std::collections::HashMap;

#[derive(Deserialize)]
struct Config {
    devices: HashMap<String, Mapping>
}


#[derive(Deserialize)]
struct  Device {

}

#[derive(Deserialize)]
struct Mapping {

}