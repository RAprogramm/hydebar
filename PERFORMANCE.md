# Performance Benchmarks

This document tracks hydebar performance metrics and optimization efforts.

## Goals (v0.8.0)

- **Binary size:** < 35MB (stripped)
- **RAM usage:** < 5MB idle, < 20MB with all modules
- **CPU usage:** < 1% idle, < 5% active
- **Startup time:** < 50ms to first paint
- **FPS:** Solid 60 FPS during animations

## Baseline Metrics (v0.6.7)

Measured on: 2025-10-08
System: Linux 6.16.10-arch1-1
Rust: 1.90.0

### Binary Size

| Metric | Size | Status |
|--------|------|--------|
| Release binary (unstripped) | 44 MB | 📊 Baseline |
| Release binary (stripped) | 34 MB | ✅ Target met |

**Analysis:**
- Stripped binary already meets < 35MB target
- Debug symbols account for ~10MB (23% overhead)
- Main contributors: iced GUI framework, Wayland protocols

### Static Analysis

| Metric | Count | Status |
|--------|-------|--------|
| Total `.clone()` calls | 342 | 🔍 Needs review |

**Top files by clone count:**
1. `hyprland_client/listeners.rs` - 36 clones
2. `network/backend/network_manager.rs` - 34 clones
3. `audio/backend.rs` - 17 clones
4. `network/backend/iwd.rs` - 16 clones
5. `modules/updates/state.rs` - 14 clones

**Categories to investigate:**
- Event listeners (Hyprland IPC)
- D-Bus service backends
- Module state updates
- Config hot-reload

### Runtime Metrics

**Note:** Runtime profiling requires running hydebar with GUI.
Metrics to be collected:
- [ ] Memory usage (heaptrack)
- [ ] CPU usage (perf)
- [ ] Startup time (hyperfine)
- [ ] Frame timing (iced metrics)

## Optimization Opportunities

### High Priority

1. **Arc instead of Clone for shared config** (Est. -20% clones)
   - Config objects cloned on hot-reload
   - Use `Arc<Config>` for read-only sharing
   - Files: `module_context.rs`, all modules

2. **Cow for string-heavy operations** (Est. -10% allocations)
   - Module names, icon names, labels
   - Use `Cow<'static, str>` where possible
   - Files: `modules/**/*.rs`

3. **Event batching in listeners** (Est. -30% CPU overhead)
   - Hyprland events: batch workspace changes
   - Network events: debounce status updates
   - Files: `listeners.rs`, `network_manager.rs`

4. **Lazy module initialization** (Est. -50ms startup)
   - Don't init disabled modules
   - Defer heavy operations (D-Bus connections)
   - Files: `modules.rs`, individual modules

### Medium Priority

5. **String interning for icons** (Est. -5% memory)
   - Icon strings duplicated across modules
   - Use static string pool
   - Files: `modules/**/*.rs`

6. **Reduce D-Bus polling** (Est. -2% CPU)
   - Use signals instead of polling where possible
   - Batch property reads
   - Files: `services/**/*.rs`

### Low Priority

7. **Optimize rendering paths** (Needs profiling)
   - Investigate iced rendering overhead
   - Potential custom widgets
   - Files: `gui/src/**/*.rs`

## Rust 1.90 Features to Leverage

**Performance-related features:**
- `#![feature(allocator_api)]` - Custom allocators
- Improved LLVM optimizations in 1.90
- Better const evaluation
- `#[inline]` hints for hot paths

**Code quality:**
- Pattern matching improvements
- Better type inference
- Lifetime elision rules

## Comparison with Waybar

**Target metrics:**

| Metric | Waybar | hydebar Goal | Status |
|--------|--------|--------------|--------|
| Binary size | ~2MB | < 35MB | ⚠️ Larger (GUI framework) |
| RAM (idle) | ~10MB | < 5MB | 🎯 TBD |
| CPU (idle) | ~2% | < 1% | 🎯 TBD |
| Startup | ~100ms | < 50ms | 🎯 TBD |

**Note:** Direct comparison challenging due to:
- Waybar: C++ with GTK
- hydebar: Rust with iced (includes Wayland compositor)
- Different feature sets

## Next Steps

### Phase 1: Measurement (Current)
- [x] Binary size baseline
- [x] Static analysis (clone count)
- [ ] Runtime profiling setup
- [ ] Establish measurement methodology

### Phase 2: Quick Wins
- [ ] Replace config clones with Arc
- [ ] Add Cow for string operations
- [ ] Implement lazy module init
- [ ] Test and measure impact

### Phase 3: Deep Optimization
- [ ] Event batching
- [ ] D-Bus optimization
- [ ] Custom allocator experiments
- [ ] Profile-guided optimization (PGO)

### Phase 4: Validation
- [ ] Automated performance tests in CI
- [ ] Regression detection
- [ ] Comparative benchmarks
- [ ] Documentation updates

## Methodology

### Binary Size
```bash
cargo build --release
ls -lh target/release/hydebar-app  # Unstripped
strip --strip-all target/release/hydebar-app -o hydebar-app-stripped
ls -lh hydebar-app-stripped  # Stripped
```

### Clone Analysis
```bash
grep -r "\.clone()" --include="*.rs" crates/ | wc -l
grep -r "\.clone()" --include="*.rs" crates/ | cut -d: -f1 | sort | uniq -c | sort -rn
```

### Memory Profiling (requires runtime)
```bash
heaptrack ./target/release/hydebar-app
heaptrack_gui heaptrack.hydebar-app.*.gz
```

### CPU Profiling (requires runtime)
```bash
perf record --call-graph dwarf ./target/release/hydebar-app
perf report
```

### Startup Time (requires runtime)
```bash
hyperfine --warmup 3 './target/release/hydebar-app'
```

## Resources

- [Rust Performance Book](https://nnethercote.github.io/perf-book/)
- [cargo-flamegraph](https://github.com/flamegraph-rs/flamegraph)
- [heaptrack](https://github.com/KDE/heaptrack)
- [perf](https://perf.wiki.kernel.org/)

---

**Last updated:** 2025-10-08
**Status:** 📊 Baseline established, optimization in progress
