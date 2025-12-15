# Summary: Data Parameters Strategy

## Overview

This strategy document collection outlines a comprehensive approach to adding arbitrary data parameters to the modular synthesizer system. The solution maintains backward compatibility, type safety, and validation while enabling rich configuration capabilities beyond the existing audio-signal-focused parameter system.

## Documentation Structure

1. **[DATA_PARAMS_STRATEGY.md](./DATA_PARAMS_STRATEGY.md)** - Main strategy document
   - Architecture overview and design rationale
   - Comparison with current system
   - Core type definitions
   - Validation approach
   - TypeScript integration
   - Migration path

2. **[DATA_PARAMS_EXAMPLES.md](./DATA_PARAMS_EXAMPLES.md)** - Implementation examples
   - Complete module examples with data params
   - DSL usage patterns
   - Migration examples from voltage encoding
   - Testing strategies
   - Advanced patterns

3. **[DATA_PARAMS_MACROS.md](./DATA_PARAMS_MACROS.md)** - Derive macro specifications
   - Attribute syntax definitions
   - Code generation templates
   - Integration with Module macro
   - Error handling specifications
   - Implementation phases

## Key Design Decisions

### 1. Parallel Parameter Systems

**Decision**: Create separate `DataParam` system alongside `InternalParam` rather than extending `InternalParam`.

**Rationale**:
- Clear separation of concerns (audio signals vs. configuration)
- No performance impact on audio thread
- Type safety without multichannel overhead
- Easier validation and tooling

### 2. Type-Safe All the Way

**Stack**:
- **Rust**: Typed fields (String, i64, f64, bool, Vec<T>)
- **Schema**: Explicit `ParamType` with constraints
- **Validation**: Type and constraint checking at patch submission
- **TypeScript**: Generated types from Rust via ts-rs
- **DSL**: Type-safe factories with autocomplete

**Benefit**: Errors caught early, excellent developer experience.

### 3. Full Module Recreation on Data Change

**Decision**: Data parameter changes trigger full module recreation rather than hot-reload.

**Rationale**:
- Data params often affect initialization (buffer sizes, lookup tables, algorithms)
- Simpler than per-module custom update logic
- Acceptable performance (data changes are rare)
- Matches existing behavior for module type changes

**Future Optimization**: Add optional `update_data_param()` method for modules that can hot-reload specific data params.

### 4. Schema-Driven Validation

**Decision**: Validation happens server-side based on schema before patch application.

**Validation Points**:
1. **Build time** (TypeScript): Type checking in DSL
2. **Patch submission** (Rust): Comprehensive validation against schema
3. **Module update** (Rust): Final type checks in `update_data()`

**Benefits**:
- Fail fast with detailed error messages
- No invalid state reaches audio thread
- Client and server stay in sync

### 5. Derive Macro for Ergonomics

**Decision**: Use `#[derive(DataParams)]` with attribute macros for field-level metadata.

**Example**:
```rust
#[derive(Default, DataParams)]
struct MyModuleData {
    #[data_param("mode", "Operating mode", enum("forward", "reverse"))]
    mode: String,
}
```

**Benefits**:
- Declarative and readable
- Compile-time validation
- Automatic schema generation
- Consistent with existing `Params` derive

## Implementation Roadmap

### Phase 1: Core Infrastructure (2-3 days)
- [ ] Add `DataParam` enum to types.rs
- [ ] Add `ParamType` enum with constraints
- [ ] Extend `ParamSchema` with type field
- [ ] Add `data` field to `ModuleState`
- [ ] Generate TypeScript types
- [ ] Add tests for serialization

### Phase 2: Validation (2 days)
- [ ] Implement `validate_data_param()` function
- [ ] Add type checking for all DataParam variants
- [ ] Add constraint validation (bounds, enums, arrays)
- [ ] Extend `validate_patch()` to check data params
- [ ] Add comprehensive validation tests

### Phase 3: Derive Macros (3-4 days)
- [ ] Implement `#[derive(DataParams)]` proc macro
- [ ] Parse `#[data_param(...)]` attributes
- [ ] Generate `get_data_state()` method
- [ ] Generate `update_data()` with validation
- [ ] Generate `get_data_schema()` method
- [ ] Update `#[derive(Module)]` to integrate data params
- [ ] Add macro tests

### Phase 4: Module System Integration (2-3 days)
- [ ] Update module construction to handle data params
- [ ] Implement module recreation on data change
- [ ] Merge audio and data param schemas
- [ ] Add name collision detection
- [ ] Update `Patch` application logic
- [ ] Integration tests

### Phase 5: TypeScript/DSL (2-3 days)
- [ ] Update type generation script
- [ ] Add `setData()` method to ModuleNode
- [ ] Implement schema-based factory generation
- [ ] Update GraphBuilder to handle data params
- [ ] Add autocomplete support
- [ ] DSL tests

### Phase 6: Example Modules (1-2 days)
- [ ] Create sample modules using data params
- [ ] Write documentation and usage examples
- [ ] Performance testing
- [ ] User-facing documentation

### Phase 7: Migration Support (1 day)
- [ ] Document migration patterns
- [ ] Create migration utilities if needed
- [ ] Update existing examples
- [ ] Backward compatibility tests

**Total Estimated Time**: 13-18 days for complete implementation

## Risk Assessment

### Low Risk
- ✅ Backward compatibility (optional field, modules work without data params)
- ✅ Performance (no audio thread impact, read-only data)
- ✅ Type safety (strong typing at all levels)

### Medium Risk
- ⚠️ Complexity in derive macros (similar to existing Params macro)
- ⚠️ TypeScript type generation (well-established with ts-rs)
- ⚠️ Schema evolution (start simple, extend later)

### Mitigations
- Extensive testing at each phase
- Incremental rollout
- Clear documentation
- Example modules as templates

## Benefits Summary

### For Module Developers
- ✅ Natural representation of configuration (enums, arrays, objects)
- ✅ Type safety in Rust code
- ✅ Automatic serialization/validation
- ✅ Clear API with derive macros

### For Patch Authors (DSL Users)
- ✅ Strongly-typed DSL with autocomplete
- ✅ Clear, readable syntax
- ✅ Immediate validation feedback
- ✅ Rich configuration options

### For System Maintainers
- ✅ Clean separation of audio and config concerns
- ✅ No performance degradation
- ✅ Easy to extend with new types
- ✅ Backward compatible

## Use Cases Enabled

1. **Wavetable Selection**: String enum for waveform types
2. **Sequencers**: Arrays of note/gate data
3. **Filters**: Enum for algorithm selection
4. **Effects**: Complex nested configuration objects
5. **Sample Players**: File path strings, loop mode booleans
6. **Quantizers**: Scale definition arrays
7. **Utilities**: Mode switches, enable/disable flags
8. **Advanced DSP**: Algorithm parameters that don't need modulation

## Alternatives Considered

### ❌ Extend InternalParam
Rejected: Pollutes audio processing, type confusion, performance overhead

### ❌ JSON Blobs in Voltages
Rejected: No type safety, unergonomic, parse overhead

### ❌ Separate Config System
Rejected: Breaks conceptual model, disconnected validation

### ✅ Parallel Parameter System (Chosen)
Advantages: Clean separation, type safe, performant, ergonomic

## Open Questions for Future Work

1. **Hot-reload optimization**: Add optional per-module data param update handlers
2. **Schema versioning**: Migration system for patches across version changes
3. **Complex constraints**: Expression language for cross-param validation
4. **Visual editors**: UI widgets generated from schema
5. **Preset system**: Serialization format for sharable configurations
6. **String interning**: Optimize memory if many modules share strings

## Success Criteria

The implementation will be considered successful when:

1. ✅ All tests pass (unit, integration, validation)
2. ✅ TypeScript types generate correctly
3. ✅ Example modules demonstrate common patterns
4. ✅ Existing patches continue to work
5. ✅ DSL provides strong typing and autocomplete
6. ✅ Validation catches errors with clear messages
7. ✅ Documentation is complete and clear
8. ✅ Performance benchmarks show no regression

## Getting Started with Implementation

When ready to implement:

1. Read all three strategy documents
2. Start with Phase 1 (Core Infrastructure)
3. Run tests after each phase
4. Update documentation as you go
5. Get feedback on DSL ergonomics early
6. Test with real-world module use cases

## Questions?

If questions arise during implementation:

- Check the examples in DATA_PARAMS_EXAMPLES.md
- Review macro specifications in DATA_PARAMS_MACROS.md
- Look at existing Params/Module macro implementations
- Consider edge cases and add tests
- Document design decisions as you make them

## Conclusion

This strategy provides a complete, type-safe solution for arbitrary data parameters in modules while maintaining the performance and ergonomics of the existing system. The parallel parameter approach clearly separates real-time audio processing from static configuration, making the codebase easier to understand and maintain while enabling powerful new module capabilities.

**No code implementation is included** - these documents serve as a comprehensive blueprint for implementation when ready to proceed.
