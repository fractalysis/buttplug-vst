#![allow(incomplete_features)]
#![feature(generic_associated_types)]
//#![feature(async_closure)]
//#![no_std]

extern crate serde;
extern crate baseplug;
extern crate buttplug;

//use core::panic::PanicInfo;
use tokio::{self, sync::mpsc, runtime::Runtime};
use serde::{Serialize, Deserialize};
use baseplug::{
    ProcessContext,
    Plugin,
};

mod buttplug_client;


baseplug::model! {
    #[derive(Debug, Serialize, Deserialize)]
    struct ButtplugModel {
        #[model(min = -90.0, max = 3.0)]
        #[parameter(name = "gain", unit = "Decibels",
            gradient = "Power(0.15)")]
        gain: f32,
    }
}

/*impl ButtplugModel {
    const SEND_RATE: f32 = 20.0; // Per second
}*/

impl Default for ButtplugModel {

    fn default() -> Self {
        Self {
            // "gain" is converted from dB to coefficient in the parameter handling code,
            // so in the model here it's a coeff.
            // -0dB == 1.0
            gain: 0.0,
        }
    }
}

struct ButtplugMonitor {
    tkrt: Runtime,
    bpio_sender: mpsc::Sender<f32>,
}

impl Plugin for ButtplugMonitor {
    const NAME: &'static str = "Buttplug Monitor";
    const PRODUCT: &'static str = "Buttplug Monitor";
    const VENDOR: &'static str = "Fractalysoft";

    const INPUT_CHANNELS: usize = 2;
    const OUTPUT_CHANNELS: usize = 2;

    type Model = ButtplugModel;

    #[inline]
    fn new(_sample_rate: f32, _model: &ButtplugModel) -> Self {
        
        let (tkrt, sender) = buttplug_client::start_buttplug_thread(20.0)
            .expect("Could not start Buttplug thread");

        ButtplugMonitor {
            tkrt,
            bpio_sender: sender,
        }
    }

    #[inline]
    fn process(&mut self, _model: &ButtplugModelProcess, ctx: &mut ProcessContext<Self>) {
        let input = &ctx.inputs[0].buffers;
        let output = &mut ctx.outputs[0].buffers;

        let mut frame_rms: f32 = 0.0;
        for i in 0..ctx.nframes {
            output[0][i] = input[0][i];
            output[1][i] = input[1][i];

            // RMS
            frame_rms += (input[0][i] + input[1][i]) / 2.0;
        }
        frame_rms = frame_rms / ctx.nframes as f32;

        // Will not block
        // DEBUG: change to RMS please
        let _ = self.bpio_sender.try_send(frame_rms );
    }
}

baseplug::vst2!(ButtplugMonitor, b"FRbm");