use crate::sound::SoundParams;
use rodio::source::Source;
use rodio::{OutputStream, OutputStreamHandle, Sink};
use std::sync::{Arc, Mutex};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PlayError {
    #[error("Audio device error: {0}")]
    Device(String),
    #[error("Playback error: {0}")]
    Playback(String),
}

pub struct Player {
    audio_context: Arc<Mutex<Option<(OutputStream, OutputStreamHandle, Sink)>>>,
}

impl Player {
    pub fn new() -> Result<Self, PlayError> {
        // Lazy initialization - audio context created only when first sound is played
        // This reduces ALSA polling errors by avoiding persistent audio connections
        Ok(Self {
            audio_context: Arc::new(Mutex::new(None)),
        })
    }

    fn ensure_audio_initialized(&self) -> Result<(), PlayError> {
        let mut context = self.audio_context
            .lock()
            .map_err(|e| PlayError::Playback(format!("Failed to lock audio context: {}", e)))?;
        
        if context.is_none() {
            // Retry logic for ALSA EINTR errors
            let mut attempts = 3;
            let (stream, handle) = loop {
                match OutputStream::try_default() {
                    Ok(result) => break result,
                    Err(e) => {
                        attempts -= 1;
                        if attempts == 0 {
                            return Err(PlayError::Device(format!("Failed to create audio output stream after retries: {}", e)));
                        }
                        // Small delay before retry to handle interrupted system calls
                        std::thread::sleep(std::time::Duration::from_millis(10));
                    }
                }
            };

            let sink = Sink::try_new(&handle)
                .map_err(|e| PlayError::Device(format!("Failed to create audio sink: {}", e)))?;
            
            *context = Some((stream, handle, sink));
        }
        
        Ok(())
    }

    pub fn play(&self, params: SoundParams) -> Result<(), PlayError> {
        self.ensure_audio_initialized()?;
        
        let mut generator = params.generator();

        let total_duration = (generator.sample.env_attack.powi(2)
            + generator.sample.env_sustain.powi(2)
            + generator.sample.env_decay.powi(2))
            * 100000.0;
        let buffer_size = total_duration.ceil() as usize;

        let mut buffer = vec![0.0; buffer_size];
        generator.generate(&mut buffer);

        let source = rodio::buffer::SamplesBuffer::new(1, 44100, buffer);

        let context = self.audio_context
            .lock()
            .map_err(|e| PlayError::Playback(format!("Failed to lock audio context: {}", e)))?;
        
        if let Some((_, handle, _)) = context.as_ref() {
            let _ = handle.play_raw(source.convert_samples());
        }

        Ok(())
    }

    pub fn append(&self, params: SoundParams) -> Result<(), PlayError> {
        self.ensure_audio_initialized()?;
        
        let mut generator = params.generator();

        let total_duration = (generator.sample.env_attack.powi(2)
            + generator.sample.env_sustain.powi(2)
            + generator.sample.env_decay.powi(2))
            * 100000.0;
        let buffer_size = total_duration.ceil() as usize;

        let mut buffer = vec![0.0; buffer_size];
        generator.generate(&mut buffer);

        let source = rodio::buffer::SamplesBuffer::new(1, 44100, buffer);

        let context = self.audio_context
            .lock()
            .map_err(|e| PlayError::Playback(format!("Failed to lock audio context: {}", e)))?;
        
        if let Some((_, _, sink)) = context.as_ref() {
            sink.append(source);
        }
        
        Ok(())
    }

    pub fn play_and_wait(&self, params: SoundParams) -> Result<(), PlayError> {
        self.append(params)?;

        let context = self.audio_context
            .lock()
            .map_err(|e| PlayError::Playback(format!("Failed to lock audio context: {}", e)))?;
        
        if let Some((_, _, sink)) = context.as_ref() {
            sink.sleep_until_end();
        }

        Ok(())
    }

    pub fn stop(&self) -> Result<(), PlayError> {
        let context = self.audio_context
            .lock()
            .map_err(|e| PlayError::Playback(format!("Failed to lock audio context: {}", e)))?;
        
        if let Some((_, _, sink)) = context.as_ref() {
            sink.stop();
        }
        
        Ok(())
    }
}
