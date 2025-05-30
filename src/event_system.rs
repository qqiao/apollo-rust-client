// src/event_system.rs

//! # Event System Core Types
//!
//! This module defines the core data structures for the event system,
//! focusing on configuration updates. It includes types for representing
//! configuration values, specific event details, and the main event enum.
//! It also defines the `EventListener` trait for consumers of these events.
//!
//! Note: The component responsible for managing listeners and dispatching these events (e.g., a Cache or a dedicated EventManager)
//! is defined elsewhere.

use wasm_bindgen::prelude::*;
use std::collections::HashMap;
use js_sys; // For js_sys::Object, js_sys::Reflect
use js_sys; // For js_sys::Object, js_sys::Reflect

/// Represents the different types a configuration value can take.
///
/// This enum is designed to be usable from JavaScript via `wasm-bindgen`.
/// Each variant holds a primitive Rust type that can be mapped to a corresponding JavaScript type.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq)]
pub enum ConfigValue {
    /// Represents a textual configuration value. Maps to a JavaScript `String`.
    String(String),
    /// Represents a numeric configuration value. Maps to a JavaScript `Number` (float).
    Number(f64),
    /// Represents a boolean configuration value. Maps to a JavaScript `Boolean`.
    Boolean(bool),
    // Future extensions could include:
    // Null,
    // Array(Vec<ConfigValue>),
    // Object(HashMap<String, ConfigValue>),
    // For now, focusing on primitive types for simplicity.
}

/// Contains detailed information about a configuration update event.
///
/// This struct specifies which namespace was affected and provides a map
/// of the configuration keys and their new values within that namespace.
/// It's designed to be efficient for Rust-side use and provides helper methods
/// for JavaScript interoperability.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq)]
pub struct ConfigUpdateEvent {
    /// The namespace of the configuration that was updated (e.g., "ui.settings", "feature_flags").
    /// This field is not directly exposed to WASM for direct field access to encourage using the getter.
    #[wasm_bindgen(skip)]
    pub namespace: String,

    /// A map of configuration keys to their new `ConfigValue`s.
    /// This map only includes keys whose values have actually changed.
    /// This field is not directly exposed to WASM; use `get_changes_as_js_value()` instead.
    #[wasm_bindgen(skip)]
    pub changes: HashMap<String, ConfigValue>,
}

#[wasm_bindgen]
impl ConfigUpdateEvent {
    /// Creates a new `ConfigUpdateEvent`.
    ///
    /// This constructor is primarily intended for Rust-side usage when an event
    /// needs to be created before being broadcast.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace identifier (e.g., "application", "feature_toggles.user_group_x").
    /// * `changes` - A `HashMap` where keys are configuration item names (String)
    ///               and values are their new `ConfigValue`. This map should represent
    ///               the actual changes to the configuration.
    pub fn new(namespace: String, changes: HashMap<String, ConfigValue>) -> Self {
        ConfigUpdateEvent { namespace, changes }
    }

    /// Returns a clone of the namespace string for this event.
    ///
    /// Exposed to JavaScript as a getter property named `namespace`.
    #[wasm_bindgen(getter)]
    pub fn namespace(&self) -> String {
        self.namespace.clone()
    }

    /// Converts the `changes` map into a JavaScript object and returns it as a `JsValue`.
    ///
    /// Each key-value pair in the Rust `HashMap<String, ConfigValue>` is translated into
    /// a property on the resulting JavaScript object. `ConfigValue` variants are converted
    /// to their corresponding JavaScript primitive types (`String`, `Number`, `Boolean`).
    ///
    /// This method is essential for allowing JavaScript listeners to easily consume
    /// the event's change payload.
    ///
    /// # Returns
    ///
    /// - `Ok(JsValue)`: A `JsValue` representing a JavaScript object containing the configuration changes.
    /// - `Err(JsValue)`: A `JsValue` (typically representing a JavaScript error) if any issues occur
    ///   during the conversion, such as errors when setting properties on the JS object.
    #[wasm_bindgen(js_name = getChanges)]
    pub fn get_changes_as_js_value(&self) -> Result<JsValue, JsValue> {
        let js_object = js_sys::Object::new();
        for (key, value) in &self.changes {
            let js_key = JsValue::from_str(key);
            let js_val = match value {
                ConfigValue::String(s) => JsValue::from_str(s),
                ConfigValue::Number(n) => JsValue::from_f64(*n),
                ConfigValue::Boolean(b) => JsValue::from_bool(*b),
            };
            js_sys::Reflect::set(&js_object, &js_key, &js_val)?;
        }
        Ok(JsValue::from(js_object))
    }
}

/// Represents various types of events that can occur within the application.
///
/// Currently, the primary event type is `ConfigUpdate`, signifying changes to
/// application configuration. The enum is `#[wasm_bindgen]` compatible, allowing
/// its type to be understood by JavaScript if `Event` objects are passed across
/// the WASM boundary, though typically interaction from JS is via methods that
/// handle specific event payloads.
#[wasm_bindgen]
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// Indicates that one or more configuration values within a specific namespace
    /// have been updated. The associated `ConfigUpdateEvent` (see its documentation)
    /// contains the namespace and a detailed map of the changes.
    ConfigUpdate(ConfigUpdateEvent),
    // Future event types could be added here, for example:
    // UserActionEvent(UserActionEventDetails),
    // SystemStatusEvent(SystemStatusDetails),
}

/// A trait for components that wish to listen to application events.
///
/// Structs implementing this trait can be registered with an event dispatching
/// mechanism (like a central `EventManager` or directly with event-producing
/// components such as a `Cache`).
///
/// # Event Handling
/// The core of this trait is the `on_event` method, which is called by the
/// event dispatcher when a relevant event occurs. Implementers should inspect
/// the provided `Event` enum to determine the event type and react accordingly.
///
/// # Thread Safety and JavaScript Listeners
/// The `Send + Sync` bounds, typically present on this trait in Rust for thread safety,
/// have been temporarily removed (see `TODO` comment). This is a pragmatic adjustment
/// to accommodate JavaScript listeners (`js_sys::Function`), which are inherently
/// not `Send` or `Sync`. This makes the system suitable for single-threaded WASM
/// environments where JS interop is common. If the Rust parts of the application
/// were to use this event system in a multi-threaded Rust context (outside of WASM's
/// main thread), this aspect would need careful reconsideration to ensure thread safety,
/// potentially by dispatching JS calls to the main thread.
// TODO: Re-evaluate Send + Sync if Cache needs to be thread-safe with JS listeners.
// For now, removing Send + Sync to allow JsEventListenerWrapper which holds js_sys::Function.
pub trait EventListener {
    /// Called by an event dispatcher when an event has occurred.
    ///
    /// Implementations should typically match on the `event` argument's variants
    /// to handle specific event types they are interested in.
    ///
    /// # Arguments
    ///
    /// * `event` - A reference to the `Event` that was broadcast.
    fn on_event(&self, event: &Event);
}
