# Nebenläufigkeit & Async

## Tokio als Async-Runtime

Für die Async-Runtime Tokio verwenden; `async-std` nur zur WASM-Kompatibilität.

Do:

```rust
#[tokio::main]
async fn main() {
    let daten = lade().await;
    verarbeite(daten).await;
}
```

Don't:

```rust
// zweite Runtime im normalen (Nicht-WASM) Build — fragmentiert das Async-Ökosystem,
// Futures/IO-Typen sind zwischen den Runtimes inkompatibel
#[async_std::main]
async fn main() {
    let daten = lade().await;
    verarbeite(daten).await;
}
```

## Message-Passing statt geteiltem Mutex

`Arc<Mutex<T>>` nur einsetzen, wenn nötig; Zustand bevorzugt per Channel-Message-Passing
teilen.

Do:

```rust
use tokio::sync::mpsc;

let (tx, mut rx) = mpsc::channel::<u64>(32);
tokio::spawn(async move {
    let mut summe = 0u64;
    while let Some(n) = rx.recv().await {
        summe += n; // ein Besitzer, keine Sperre
    }
});
tx.send(5).await.unwrap();
```

Don't:

```rust
use std::sync::{Arc, Mutex};

let zähler = Arc::new(Mutex::new(0u64));
for _ in 0..8 {
    let z = Arc::clone(&zähler);
    std::thread::spawn(move || {
        *z.lock().unwrap() += 1; // Lock-Contention unter Last + Poisoning bei Panik im Critical Section
    });
}
```
