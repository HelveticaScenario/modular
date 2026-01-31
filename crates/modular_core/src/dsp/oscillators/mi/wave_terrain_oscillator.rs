//! Wave terrain synthesis based on Mutable Instruments Plaits.

crate::mi_engine_module! {
    name: "mi.waveterrain",
    doc: "Wave terrain synthesis - A 2D function evaluated along an elliptical path of adjustable center and eccentricity",
    struct_name: WaveTerrainOscillator,
    engine_type: WaveTerrainEngine<'a>,
    engine_path: mi_plaits_dsp::engine2::wave_terrain_engine::WaveTerrainEngine,
    constructor: new(BLOCK_SIZE),
    output_doc: "direct terrain height (z)",
    aux_doc: "terrain height interpreted as phase distortion (sin(y+z))",
    params: {
        freq: "frequency in v/oct",
        timbre: "path radius on the terrain",
        morph: "path offset on the terrain",
        harmonics: "terrain selection",
        sync: "sync/trigger input (expects >0V to trigger)",
    }
}
