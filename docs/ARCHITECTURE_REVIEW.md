# MudOS/mudlib Architecture Review & Improvement Plan

## Executive Summary

After reviewing [FluffOS](https://www.fluffos.info/) and [MudOS v21c2](https://documentation.help/MudOS-v21c2-zh/chapter21.html), this document outlines the architectural improvements needed to achieve **driver/mudlib separation** and **hot-reload capability** similar to LPMUD.

---

## 1. MudOS Architecture Overview

### 1.1 Core Components

```
┌─────────────────────────────────────────────────────────────┐
│                        Driver (C)                           │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Network      │  │ LPC          │  │ Event Loop   │     │
│  │ (TCP/TLS/WS) │  │ Interpreter  │  │ (Commands/   │     │
│  │              │  │              │  │  Heartbeat)  │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            │
                    ┌───────┴────────┐
                    │  Master Object │ ← Central Coordinator
                    │  (mudlib)      │
                    └────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼──────┐   ┌────────▼─────┐   ┌────────▼─────┐
│    Rooms     │   │    Objects   │   │   Mobs/NPCs  │
│  (mudlib)    │   │   (mudlib)   │   │   (mudlib)   │
└──────────────┘   └──────────────┘   └──────────────┘
```

### 1.2 Key MudOS Features

| Feature | Description |
|---------|-------------|
| **Master Object** | Central coordinator with applies: `valid_compile`, `error_handler`, `connect`, `crash` |
| **Applies** | Driver callbacks: `create()`, `reset()`, `init()`, `heart_beat()`, `move_or_destruct()` |
| **Efuns** | External functions from driver: `move_object()`, `this_player()`, `call_out()`, `reload_object()` |
| **Hot Reload** | `reload_object()` - reload object code at runtime without restart |
| **Shadowing** | Ability to override object methods dynamically |
| **Virtual Objects** | Compile objects on-demand from templates |

---

## 2. Current Architecture Analysis

### 2.1 What We Have

```
┌─────────────────────────────────────────────────────────────┐
│                    Rust Application                         │
│  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │
│  │ Tokio TCP    │  │  Rhai        │  │ Game Loop    │     │
│  │ Server       │  │  Scripts     │  │ (1 sec tick) │     │
│  └──────────────┘  └──────────────┘  └──────────────┘     │
└─────────────────────────────────────────────────────────────┘
                            │
                    ┌───────▼────────┐
                    │ Command Registry│
                    │ (Static)       │
                    └────────────────┘
                            │
        ┌───────────────────┼───────────────────┐
        │                   │                   │
┌───────▼──────┐   ┌────────▼─────┐   ┌────────▼─────┐
│ cmds/*.rhai  │   │  Player Data │   │   Object     │
│ (Hot Reload) │   │  (HashMap)   │   │   System     │
└──────────────┘   └──────────────┘   └──────────────┘
```

### 2.2 Gaps Compared to MudOS

| Gap | Impact | Priority |
|-----|--------|----------|
| **No Master Object** | No central coordination between driver and mudlib | HIGH |
| **Limited Hot Reload** | Only scripts reload, not object definitions | HIGH |
| **No Shadowing** | Cannot override methods dynamically | MEDIUM |
| **No Virtual Objects** | All objects must be pre-defined | MEDIUM |
| **Static Command Registry** | Commands registered at startup only | LOW |
| **No call_out()** | No delayed function scheduling | HIGH |
| **No heart_beat per object** | Only global game loop | MEDIUM |

---

## 3. Improvement Plan

### 3.1 Master Object Pattern (HIGH PRIORITY)

Create a Master Object that acts as the interface between driver (Rust) and mudlib (Rhai):

```rust
// src/master/mod.rs
pub struct MasterObject {
    script_storage: Arc<RwLock<ScriptStorage>>,
}

impl MasterObject {
    // Applies - called by driver at specific events
    pub fn connect(&self, player: &Body) -> Result<(), String>;
    pub fn error_handler(&self, error: &str) -> Result<(), String>;
    pub fn valid_compile(&self, path: &str) -> bool;
    pub fn crash(&self, error: &str);

    // Object lifecycle applies
    pub fn create(&self, object: &mut Object) -> Result<(), String>;
    pub fn reset(&self, object: &mut Object);
    pub fn init(&self, object: &Object, player: &Body);
}
```

**Rhai Master Script** (`master.rhai`):
```rhai
// master.rhai - Master object for driver/mudlib coordination

fn connect(player_name) {
    // Called when player connects
    // Return login object or error message
}

fn error_handler(error, source) {
    // Log error to file
    // Return true to continue, false to shutdown
}

fn valid_compile(path) {
    // Check if file can be compiled
    // Used for security checks
}

fn create(obj) {
    // Called when object is first created
}

fn reset(obj) {
    // Called periodically to refresh objects
}
```

### 3.2 Object Reload System (HIGH PRIORITY)

Implement `reload_object()` equivalent:

```rust
// src/object/reload.rs
pub fn reload_object(
    storage: &ScriptStorage,
    object_id: &str,
) -> Result<(), Box<dyn Error>> {
    // 1. Find all instances of object
    // 2. Save current state
    // 3. Reload script from disk
    // 4. Restore state
    // 5. Notify connected players
}

// Rhai API
engine.register_fn("reload_object", |name: &str| -> bool {
    // Reload object definition
    // Return true if successful
});
```

### 3.3 Call Out System (HIGH PRIORITY)

Implement delayed function calls:

```rust
// src/scheduler/call_out.rs
pub struct CallOutScheduler {
    tasks: HashMap<uuid::String, CallOutTask>,
}

pub struct CallOutTask {
    id: String,
    target: String,  // object path
    function: String,
    delay: Duration,
    args: Vec<Dynamic>,
    scheduled_at: Instant,
}

impl CallOutScheduler {
    pub fn call_out(&mut self, target: &str, func: &str, delay: Duration, args: Vec<Dynamic>);
    pub fn remove_call_out(&mut self, target: &str, func: &str) -> bool;
    pub fn process_due(&mut self) -> Vec<CallOutResult>;
}
```

**Rhai API**:
```rhai
// Schedule a function call in 10 seconds
call_out("heart_beat", 10);

// Remove scheduled call
remove_call_out("heart_beat");
```

### 3.4 Heart Beat per Object (MEDIUM PRIORITY)

Allow individual objects to have heart beats:

```rust
// src/scheduler/heart_beat.rs
pub struct HeartBeatRegistry {
    objects: HashSet<String>,  // Object IDs with heart beat
}

impl HeartBeatRegistry {
    pub fn set_heart_beat(&mut self, object_id: &str, enabled: bool);
    pub fn process_all(&self, world: &mut World);
}
```

**Rhai API**:
```rhai
// Enable heart beat for this object
set_heart_beat(true);

// In object's script
fn heart_beat() {
    // Called every second
    // Combat, regeneration, etc.
}
```

### 3.5 Virtual Objects (MEDIUM PRIORITY)

Compile objects on-demand from templates:

```rust
// src/object/virtual.rs
pub fn compile_virtual(
    template: &str,
    params: HashMap<String, Dynamic>,
) -> Result<Arc<Object>, Box<dyn Error>> {
    // 1. Load template script
    // 2. Substitute parameters
    // 3. Compile and cache
    // 4. Return new instance
}
```

**Example** (`rooms/virtual/zone.rhai`):
```rhai
// Virtual room template
fn create_template(zone, room_num) {
    // Generate room dynamically
    let desc = get_room_desc(zone, room_num);
    let exits = get_room_exits(zone, room_num);
    // ...
}
```

---

## 4. Implementation Roadmap

### Phase 1: Master Object (Week 1-2)
- [ ] Create `src/master/mod.rs`
- [ ] Implement basic applies: `connect`, `error_handler`
- [ ] Create `master.rhai` script
- [ ] Integrate with server initialization

### Phase 2: Call Out System (Week 2-3)
- [ ] Create `src/scheduler/call_out.rs`
- [ ] Implement task queue
- [ ] Add Rhai API: `call_out()`, `remove_call_out()`
- [ ] Integrate with game loop

### Phase 3: Object Reload (Week 3-4)
- [ ] Create `src/object/reload.rs`
- [ ] Implement state preservation
- [ ] Add Rhai API: `reload_object()`
- [ ] Test with running game

### Phase 4: Heart Beat Registry (Week 4-5)
- [ ] Create `src/scheduler/heart_beat.rs`
- [ ] Implement per-object heart beats
- [ ] Add Rhai API: `set_heart_beat()`
- [ ] Migrate from global game loop

### Phase 5: Virtual Objects (Week 5-6)
- [ ] Create `src/object/virtual.rs`
- [ ] Implement template system
- [ ] Add caching layer
- [ ] Create example templates

---

## 5. File Structure Changes

```
src/
├── master/
│   ├── mod.rs           # Master object coordinator
│   └── applies.rs       # Apply callbacks
├── scheduler/
│   ├── mod.rs           # Scheduler module
│   ├── call_out.rs      # Delayed function calls
│   └── heart_beat.rs    # Per-object heart beats
├── object/
│   ├── mod.rs           # Object module
│   ├── base.rs          # (existing)
│   ├── reload.rs        # Object reload system
│   └── virtual.rs       # Virtual object templates
└── hotreload/
    ├── mod.rs           # Hot reload coordinator
    └── watcher.rs       # File system watcher

cmds/
├── master.rhai          # Master object script
├── login.rhai           # Login handler
└── virtual/             # Virtual object templates
    ├── room.rhai
    └── mob.rhai
```

---

## 6. Testing Strategy

### 6.1 Unit Tests
- Master object applies
- Call out scheduling
- Object reload state preservation
- Heart beat registry

### 6.2 Integration Tests
- Hot reload during active game
- Call out execution
- Virtual object compilation

### 6.3 Load Tests
- 1000+ heart beats per second
- 10000+ scheduled call outs
- Rapid object reload cycles

---

## 7. Migration Notes

### 7.1 Backward Compatibility
- All existing `.rhai` scripts continue to work
- New features are opt-in
- Gradual migration path

### 7.2 Configuration
```toml
# config/mud.toml
[driver]
master = "cmds/master.rhai"
enable_hot_reload = true
enable_virtual_objects = true

[scheduler]
heart_beat_interval = 1
call_out_resolution = 0.1
```

---

## 8. References

- [FluffOS Documentation](https://www.fluffos.info/)
- [MudOS v21c2 Documentation](https://documentation.help/MudOS-v21c2-zh/chapter21.html)
- [LPMUD FAQ](https://github.com/maldorne/awesome-muds)
- [Dead Souls Mudlib](http://dead-souls.net/ds-creator-faq.html)
