/// Apply the existing stock-weapon rules to known reskins without changing the export name.
pub(super) fn canonical_projectile_weapon(weapon: &str) -> &str {
    if weapon.eq_ignore_ascii_case("quake_rl") {
        "rocketlauncher"
    } else {
        weapon
    }
}

/// Exact-name tags shared by individual kills and scored groups.
pub(super) fn projectile_tags(weapon: &str) -> &'static [&'static str] {
    match weapon {
        "rocketlauncher" | "directhit" | "blackbox" | "liberty_launcher" | "airstrike" => {
            &["projectile_kill", "rocket"]
        }
        "grenadelauncher" | "loch_n_load" | "iron_bomber" => &["projectile_kill", "pipe"],
        "stickybomb_launcher" | "quickiebomb_launcher" | "flaregun" | "detonator"
        | "scorch_shot" | "compound_bow" | "crusaders_crossbow" | "syringegun_medic"
        | "rescue_ranger" | "righteous_bison" | "loose_cannon" | "loose_cannon_impact"
        | "loose_cannon_explosion" => &["projectile_kill"],
        _ => &[],
    }
}
