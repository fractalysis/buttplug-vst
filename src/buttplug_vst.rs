// author: doomy <alexander@resamplr.com>

#[macro_use]
extern crate vst;
extern crate time;

use buttplug_client::start_buttplug_thread;
use vst::buffer::AudioBuffer;
use vst::plugin::{Category, HostCallback, Info, Plugin, PluginParameters};
use vst::util::AtomicFloat;
use tokio::sync::mpsc;
use std::sync::Arc;
use tokio::runtime::Runtime;

mod buttplug_client;

const SEND_RATE: f32 = 20.0;

/// Simple Gain Effect.
/// Note that this does not use a proper scale for sound and shouldn't be used in
/// a production amplification effect!  This is purely for demonstration purposes,
/// as well as to keep things simple as this is meant to be a starting point for
/// any effect.
struct Buttplugin {
    // Store a handle to the plugin's parameter object.
    params: Arc<GainEffectParameters>,
    tkrt: Runtime,
    bpio_sender: mpsc::Sender<f32>,
}

/// The plugin's parameter object contains the values of parameters that can be
/// adjusted from the host.  If we were creating an effect that didn't allow the
/// user to modify it at runtime or have any controls, we could omit this part.
///
/// The parameters object is shared between the processing and GUI threads.
/// For this reason, all mutable state in the object has to be represented
/// through thread-safe interior mutability. The easiest way to achieve this
/// is to store the parameters in atomic containers.
struct GainEffectParameters {
    // The plugin's state consists of a single parameter: amplitude.
    amplitude: AtomicFloat,
}

impl Default for GainEffectParameters {
    fn default() -> GainEffectParameters {
        GainEffectParameters {
            amplitude: AtomicFloat::new(0.5),
        }
    }
}

impl PluginParameters for GainEffectParameters {
    // the `get_parameter` function reads the value of a parameter.
    fn get_parameter(&self, index: i32) -> f32 {
        match index {
            0 => self.amplitude.get(),
            _ => 0.0,
        }
    }

    // the `set_parameter` function sets the value of a parameter.
    fn set_parameter(&self, index: i32, val: f32) {
        #[allow(clippy::single_match)]
        match index {
            0 => self.amplitude.set(val),
            _ => (),
        }
    }

    // This is what will display underneath our control.  We can
    // format it into a string that makes the most since.
    fn get_parameter_text(&self, index: i32) -> String {
        match index {
            0 => format!("{:.2}", (self.amplitude.get() - 0.5) * 2f32),
            _ => "".to_string(),
        }
    }

    // This shows the control's name.
    fn get_parameter_name(&self, index: i32) -> String {
        match index {
            0 => "Amplitude",
            _ => "",
        }
        .to_string()
    }
}



impl Default for Buttplugin {
    fn default() -> Buttplugin {
        Buttplugin {
            params: Arc::new(GainEffectParameters::default()),
            tkrt: Runtime::new().unwrap(),
            bpio_sender: mpsc::channel( 1 ).0,
        }
    }
}

// All plugins using `vst` also need to implement the `Plugin` trait.  Here, we
// define functions that give necessary info to our host.
impl Plugin for Buttplugin {

    fn new(_host: HostCallback) -> Self {

        let (tkrt, sender) = start_buttplug_thread(SEND_RATE, 72000.0)
            .expect("Could not start Buttplug thread");

        Buttplugin {
            params: Arc::new(GainEffectParameters::default()),
            tkrt,
            bpio_sender: sender,
        }
    }

    fn get_info(&self) -> Info {
        Info {
            name: "Ahhahaha".to_string(),
            vendor: "Fraccy".to_string(),
            unique_id: 696969,
            version: 1,
            inputs: 2,
            outputs: 2,
            // This `parameters` bit is important; without it, none of our
            // parameters will be shown!
            parameters: 1,
            category: Category::Effect,
            ..Default::default()
        }
    }

    // Here is where the bulk of our audio processing code goes.
    fn process(&mut self, buffer: &mut AudioBuffer<f32>) {
        
        for i in 0..buffer.samples() {
            for (input, output) in buffer.zip() {
                output[i] = input[i];

                // Send the value to the buttplug thread
                let _ = self.bpio_sender.try_send(input[i]);
            }
        }

    }

    // Return the parameter object. This method can be omitted if the
    // plugin has no parameters.
    fn get_parameter_object(&mut self) -> Arc<dyn PluginParameters> {
        Arc::clone(&self.params) as Arc<dyn PluginParameters>
    }
}

// This part is important!  Without it, our plugin won't work.
plugin_main!(Buttplugin);