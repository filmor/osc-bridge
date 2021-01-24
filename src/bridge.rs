use std::collections::HashMap;

use crate::osc_device::OscDevice;


struct Bridge {
    devices: HashMap<String, OscDevice>,
    
}