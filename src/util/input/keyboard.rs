use winit::event::{DeviceEvent, ElementState, KeyboardInput, ModifiersState, VirtualKeyCode};

// Issues with this code:
//  1. The "pressed" duration is measured in FRAMES, which is stupidly device/context-dependent.
//  2. Key presses are not registered until `Keyboard::tick(&mut keyboard)` is called, practically
//     meaning the class cannot be used outside MainEventsCleared.
//  3. It suffers from the age-old problem of, if you manage to both press and release a key quick
//     enough that the event for both is received before tick(), then it's as if you never pressed
//     the key at all. This has been actually observed and isn't just a theoretical issue...
//  4. It does use more storage than is most likely necessary. Probably doesn't matter.
//
// Issue with 1. is that in order for it not to be frame count dependent, the keyboard must
// somehow be aware of time, but cluttering the interface by requiring current time in seconds is
// not desirable. You could pass the current time to the Keyboard in `tick()`, but then you're
// stuck with 2.
//
// 2. has been an issue in practice with the main menu, but it ended up being fairly easy to work
// around by not using Keyboard at all (!), so it's unclear whether fixing this is a good idea,
// especially if it takes long or makes this otherwise harder/clunkier to use.
//
// 3. should definitely be fixed. Probably just requires staring at the current logic for a bit.
//
// 4. can probably be ignored.

pub type Mods = ModifiersState;

pub struct Keyboard {
    pressed: Box<[u32]>,              // key index -> "`frame count when pressed` & 0xFFFF"
    just_released: Box<[(u32, u32)]>, // key index -> ("number of frames pressed", "frame count when released")
    frame_counter: u32,               // incremented once after all events for the frame have been received
}

pub type Key = VirtualKeyCode;

impl Keyboard {
    /// Resets the keyboard state to as if it was just freshly created and no events have been
    /// received so far.
    pub fn clear_all(&mut self) {
        self.pressed.fill(0);
        self.just_released.fill((0, 0));
    }

    /// Returns:
    /// * `1` if `positive_key` is pressed but `negative_key` is NOT pressed
    /// * `0` if neither or both of the keys are pressed
    /// * `-1` if `positive_key` is NOT pressed but `negative_key` is pressed
    /// 
    /// For example, forward-backward movement can be implemented as
    /// ```rs
    /// position += forward_dir * keyboard.get_axis(Key::W, Key::S) as f32;
    /// ```
    pub fn get_axis(&self, positive_key: Key, negative_key: Key) -> i32 {
        self.pressed(positive_key) as i32 - self.pressed(negative_key) as i32
    }

    /// Returns true if the key is currently pressed.
    pub fn pressed(&self, key: Key) -> bool {
        self.pressed_frames(key) > 0
    }

    /// Returns the number of frames the key has been pressed,
    /// or zero if it is not currently pressed.
    pub fn pressed_frames(&self, key: Key) -> u32 {
        let timestamp = self.pressed[key as usize];
        if timestamp == 0 {
            0
        } else {
            self.frame_counter - timestamp
        }
    }

    /// Returns true if the key is pressed, and the specified
    /// modifier keys were pressed when the key was first pressed.
    /// 
    /// * Note that this means you can't first press the key and THEN
    ///   the modifier keys - this will return false in such case.
    /// * This will also return false if the modifier keys were pressed
    ///   down before `key` BUT released by now, even if `key` is still pressed.
    pub fn pressed_with_mods(&self, key: Key, mods: Mods) -> bool {
        self.pressed_frames_with_mods(key, mods) > 0
    }

    /// If the specified modifier keys were pressed before `key` was pressed,
    /// returns the number of frames `key` has been down; otherwise returns zero.
    /// 
    /// * Note that this means you can't first press the key and THEN
    ///   the modifier keys - this will return `0` in such case.
    /// * This will also return `0` if the modifier keys were pressed
    ///   down before `key` BUT released by now, even if `key` is still pressed.
    pub fn pressed_frames_with_mods(&self, key: Key, mods: Mods) -> u32 {
        let ticks_down = self.pressed_frames(key);
        if ticks_down == 0 {
            return 0;
        }
        // Logic here is that you usually have to press a modifier key *before* you press
        // the key you want to apply it to. You wouldn't press 'S + ctrl' to save, but 'ctrl + S'.
        // Therefore I'm requiring the modifiers to have been held down longer than the key.
        if mods.ctrl() && self.pressed_frames(Key::LControl) < ticks_down {
            return 0;
        }
        if mods.alt() && self.pressed_frames(Key::LAlt) < ticks_down {
            return 0;
        }
        if mods.shift() && self.pressed_frames(Key::LShift) < ticks_down {
            return 0;
        }
        ticks_down
    }

    /// Returns true if the key was pressed between the previous frame
    /// and this point in time.
    pub fn just_pressed(&self, key: Key) -> bool {
        self.pressed_frames(key) == 1
    }

    /// Returns true if the key was pressed between the previous frame
    /// and this point in time, and the specified modifier keys were
    /// down when the key was pressed.
    pub fn just_pressed_with_mods(&self, key: Key, mods: Mods) -> bool {
        self.pressed_frames_with_mods(key, mods) == 1
    }

    /// Returns true if the key was just released, and had been
    /// held down for a very short amount of time (a "tap").
    pub fn tapped(&self, key: Key) -> bool {
        self.tapped_with_threshold(key, 7)
    }

    /// Returns true if the key was just released and had been
    /// held down for less than the specified number of *frames*.
    pub fn tapped_with_threshold(&self, key: Key, max_frames: u32) -> bool {
        self.just_released_frames(key) <= max_frames
    }

    /// Returns `true` if the key was released between the previous frame
    /// and this point point time.
    pub fn just_released(&self, key: Key) -> bool {
        self.just_released_frames(key) > 0
    }

    /// If the key was released between the previous frame and this point
    /// in time, the number of frames the key was pressed will be returned.
    /// Otherwise returns zero.
    /// Intended for things like "the longer you hold down this key with
    /// this ability the more powerful it will be" as this requires knowing how
    /// long it had been charging.
    pub fn just_released_frames(&self, key: Key) -> u32 {
        let (frame_count, check) = self.just_released[key as usize];
        if check != self.frame_counter {
            0
        } else {
            frame_count
        }
    }

    /// Releases the key and returns true if it was actually even pressed.
    pub fn release(&mut self, key: Key) -> bool {
        self.release_get_frames(key) > 0
    }

    /// Releases the key and gets the number of frames the key has been 
    /// pressed, or 0 if it wasn't pressed.
    pub fn release_get_frames(&mut self, key: Key) -> u32 {
        let frames = self.pressed_frames(key);
        self.pressed[key as usize] = 0;
        frames
    }
}

impl Keyboard {
    pub fn new() -> Self {
        let mut pressed = Vec::new();
        pressed.resize(256, 0);

        let mut just_released = Vec::new();
        just_released.resize(256, (0, 0));

        Self {
            pressed: pressed.into_boxed_slice(),
            just_released: just_released.into_boxed_slice(),
            frame_counter: 0,
        }
    }

    // Returns false if event not consumed
    pub fn handle_key_event(keyboard: &mut Keyboard, event: &DeviceEvent) -> bool {
        if let &DeviceEvent::Key(KeyboardInput {
            virtual_keycode: Some(key),
            state,
            ..
        }) = event
        {
            match state {
                ElementState::Pressed => {
                    // Winit does not distinguish between 'Pressed' and 'Repeat',
                    // and frame counting breaks if repeat is not filtered out, so
                    // check first that the key has actually been released before re-assigning.
                    // Allow repeat in text mode though
                    if keyboard.pressed[key as usize] == 0 {
                        keyboard.pressed[key as usize] = keyboard.frame_counter;
                    }
                }
                ElementState::Released => {
                    let frames_pressed = keyboard.pressed_frames(key);
                    keyboard.pressed[key as usize] = 0;
                    keyboard.just_released[key as usize] = (frames_pressed, keyboard.frame_counter);
                }
            }
            return true;
        }
        false
    }

    /// Should be called after all input events have been received, but
    /// before use (so right at the start of MainEventsCleared): 
    /// none of the received events will be registered before this is called!
    pub fn tick(keyboard: &mut Keyboard) {
        keyboard.frame_counter += 1;
    }
}
