use async_trait::async_trait;
use serde_json::Value;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::RwLock; // Using tokio's RwLock as it's common in async Rust and works well.

/// Event representing a configuration change for a namespace.
#[derive(Debug, Clone)]
pub struct ConfigurationChangeEvent {
    pub namespace_name: String,
    pub old_configuration: Option<Value>,
    pub new_configuration: Value,
}

/// Trait for observers that want to be notified of configuration changes.
#[async_trait]
pub trait Observer: Send + Sync {
    async fn on_configuration_change(&self, event: &ConfigurationChangeEvent);
}

/// Manages registration of observers and notification of events.
#[derive(Default)] // Removed Debug to avoid issues with dyn Observer
pub struct EventManager {
    observers: RwLock<HashMap<String, Vec<Arc<dyn Observer>>>>,
}

impl EventManager {
    pub fn new() -> Self {
        EventManager {
            observers: RwLock::new(HashMap::new()),
        }
    }

    /// Registers an observer for a given namespace.
    pub async fn register_observer(&self, namespace: &str, observer: Arc<dyn Observer>) {
        let mut observers_guard = self.observers.write().await;
        observers_guard
            .entry(namespace.to_string())
            .or_default()
            .push(observer);
    }

    /// Unregisters an observer for a given namespace.
    /// This implementation removes all occurrences of the observer if it was registered multiple times.
    /// A more precise unregistration might require observers to be identifiable (e.g., via an ID).
    pub async fn unregister_observer(&self, namespace: &str, observer_to_remove: Arc<dyn Observer>) {
        let mut observers_guard = self.observers.write().await;
        if let Some(namespace_observers) = observers_guard.get_mut(namespace) {
            // Arc<dyn Observer> doesn't directly support PartialEq, so we compare pointers.
            // This means it only unregisters the exact same Arc instance.
            namespace_observers.retain(|obs| !Arc::ptr_eq(obs, &observer_to_remove));
        }
    }

    /// Notifies all relevant observers about a configuration change event.
    pub async fn notify_observers(&self, event: ConfigurationChangeEvent) {
        let observers_guard = self.observers.read().await;
        if let Some(namespace_observers) = observers_guard.get(&event.namespace_name) {
            for observer in namespace_observers {
                // Clone Arc for each task if observers can be called concurrently
                // or if the on_configuration_change itself is long-running.
                // For simplicity here, direct call. Consider spawning tasks if needed.
                let obs_clone = observer.clone();
                let event_clone = event.clone(); // Clone event data for each observer
                
                // Spawning a new task for each observer call to prevent one observer
                // from blocking others. This is important for WASM as well,
                // to yield execution.
                cfg_if::cfg_if! {
                    if #[cfg(target_arch = "wasm32")] {
                        wasm_bindgen_futures::spawn_local(async move {
                            obs_clone.on_configuration_change(&event_clone).await;
                        });
                    } else {
                        tokio::spawn(async move { // Assuming tokio runtime for non-WASM
                            obs_clone.on_configuration_change(&event_clone).await;
                        });
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_std::task;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use serde_json::json;

    // Mock Observer
    #[derive(Debug)]
    struct MockObserver {
        call_count: Arc<AtomicUsize>,
        last_event_namespace: Arc<RwLock<Option<String>>>,
    }

    impl MockObserver {
        fn new() -> Self {
            MockObserver {
                call_count: Arc::new(AtomicUsize::new(0)),
                last_event_namespace: Arc::new(RwLock::new(None)),
            }
        }
    }

    #[async_trait]
    impl Observer for MockObserver {
        async fn on_configuration_change(&self, event: &ConfigurationChangeEvent) {
            self.call_count.fetch_add(1, Ordering::SeqCst);
            let mut last_ns = self.last_event_namespace.write().await;
            *last_ns = Some(event.namespace_name.clone());
            // Simulate some async work
            task::sleep(std::time::Duration::from_millis(10)).await;
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_register_and_notify() {
        let event_manager = Arc::new(EventManager::new());
        let observer1 = Arc::new(MockObserver::new());
        let namespace = "test_namespace";

        event_manager.register_observer(namespace, observer1.clone()).await;

        let event = ConfigurationChangeEvent {
            namespace_name: namespace.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "value"}),
        };
        event_manager.notify_observers(event).await;

        // Give some time for async notification to complete
        task::sleep(std::time::Duration::from_millis(100)).await; 
        assert_eq!(observer1.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(*observer1.last_event_namespace.read().await, Some(namespace.to_string()));
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_unregister_observer() {
        let event_manager = Arc::new(EventManager::new());
        let observer1 = Arc::new(MockObserver::new());
        let namespace = "test_unregister";

        event_manager.register_observer(namespace, observer1.clone()).await;
        event_manager.unregister_observer(namespace, observer1.clone()).await;

        let event = ConfigurationChangeEvent {
            namespace_name: namespace.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "unregistered"}),
        };
        event_manager.notify_observers(event).await;
        
        task::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(observer1.call_count.load(Ordering::SeqCst), 0);
    }

    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_multiple_observers() {
        let event_manager = Arc::new(EventManager::new());
        let observer1 = Arc::new(MockObserver::new());
        let observer2 = Arc::new(MockObserver::new());
        let namespace = "test_multiple";

        event_manager.register_observer(namespace, observer1.clone()).await;
        event_manager.register_observer(namespace, observer2.clone()).await;

        let event = ConfigurationChangeEvent {
            namespace_name: namespace.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "multiple_observers"}),
        };
        event_manager.notify_observers(event).await;

        task::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(observer1.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(observer2.call_count.load(Ordering::SeqCst), 1);
    }
    
    #[cfg(not(target_arch = "wasm32"))]
    #[tokio::test]
    async fn test_namespace_specificity() {
        let event_manager = Arc::new(EventManager::new());
        let observer_ns1 = Arc::new(MockObserver::new());
        let observer_ns2 = Arc::new(MockObserver::new());
        let namespace1 = "namespace1";
        let namespace2 = "namespace2";

        event_manager.register_observer(namespace1, observer_ns1.clone()).await;
        event_manager.register_observer(namespace2, observer_ns2.clone()).await;

        let event_ns1 = ConfigurationChangeEvent {
            namespace_name: namespace1.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "ns1_event"}),
        };
        event_manager.notify_observers(event_ns1).await;
        
        task::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(observer_ns1.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(observer_ns2.call_count.load(Ordering::SeqCst), 0);

        let event_ns2 = ConfigurationChangeEvent {
            namespace_name: namespace2.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "ns2_event"}),
        };
        event_manager.notify_observers(event_ns2).await;

        task::sleep(std::time::Duration::from_millis(100)).await;
        assert_eq!(observer_ns1.call_count.load(Ordering::SeqCst), 1); // Should not have changed
        assert_eq!(observer_ns2.call_count.load(Ordering::SeqCst), 1);
    }

    // WASM specific tests
    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;
    
    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_test_configure!(run_in_browser);

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn test_register_and_notify_wasm() {
        let event_manager = Arc::new(EventManager::new());
        let observer1 = Arc::new(MockObserver::new());
        let namespace = "test_namespace_wasm";

        event_manager.register_observer(namespace, observer1.clone()).await;

        let event = ConfigurationChangeEvent {
            namespace_name: namespace.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "value_wasm"}),
        };
        event_manager.notify_observers(event).await;

        // Yield for a bit to allow spawn_local tasks to run
        gloo_timers::future::TimeoutFuture::new(100).await;
        assert_eq!(observer1.call_count.load(Ordering::SeqCst), 1);
        assert_eq!(*observer1.last_event_namespace.read().await, Some(namespace.to_string()));
    }

    #[cfg(target_arch = "wasm32")]
    #[wasm_bindgen_test]
    async fn test_unregister_observer_wasm() {
        let event_manager = Arc::new(EventManager::new());
        let observer1 = Arc::new(MockObserver::new());
        let namespace = "test_unregister_wasm";

        event_manager.register_observer(namespace, observer1.clone()).await;
        event_manager.unregister_observer(namespace, observer1.clone()).await;

        let event = ConfigurationChangeEvent {
            namespace_name: namespace.to_string(),
            old_configuration: None,
            new_configuration: json!({"key": "unregistered_wasm"}),
        };
        event_manager.notify_observers(event).await;
        
        gloo_timers::future::TimeoutFuture::new(100).await;
        assert_eq!(observer1.call_count.load(Ordering::SeqCst), 0);
    }
}
