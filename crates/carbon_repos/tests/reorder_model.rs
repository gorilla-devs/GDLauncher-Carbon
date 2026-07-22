//! Model-based execution tests for the instance/server reordering queries.
//!
//! Every operation is applied to BOTH the real database (through the repo
//! fns, real binds, real pool) and an in-memory model of the tables; after
//! each step the full table state is compared. Randomized layouts plus
//! boundary-heavy parameters drive the range arithmetic of the `shift_*`
//! family, so an off-by-one in any WHERE clause, a missing group scope, or a
//! mistyped bind diverges from the model within a few steps.
//!
//! Seeds are fixed, so failures reproduce exactly; every assertion message
//! carries `(seed, step, op)`.

use carbon_repos::db_exec::Db;
use carbon_repos::dbtypes::{DbDateTime, from_millis};
use carbon_repos::repos::instance as inst;
use carbon_repos::repos::server as srv;
use rusqlite::Connection;

// ---------------------------------------------------------------------------
// Deterministic RNG (SplitMix64) — no external dev-dependency.
// ---------------------------------------------------------------------------

struct Rng(u64);

impl Rng {
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
    fn int(&mut self, lo: i32, hi: i32) -> i32 {
        lo + self.below((hi - lo + 1) as u64) as i32
    }
    fn flag(&mut self) -> bool {
        self.below(2) == 1
    }
}

/// Migrated tempfile DB opened through the real pool, foreign keys ON to
/// match the production `db_bootstrap` configuration (`Instance.groupId` /
/// `Server.groupId` are `ON DELETE RESTRICT`, so group deletes in the op mix
/// only ever target empty groups, like the managers do).
async fn migrated_db() -> (tempfile::TempDir, Db) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("t.db");
    {
        let mut conn = Connection::open(&path).unwrap();
        let (m, _n) = carbon_repos::get_migrations();
        m.to_latest(&mut conn).unwrap();
    }
    let db = Db::open(&path, 2, true).unwrap();
    (dir, db)
}

// ---------------------------------------------------------------------------
// Instance family
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MInst {
    id: i32,
    name: String,
    shortpath: String,
    favorite: bool,
    has_pack_update: bool,
    index: i32,
    lib: Option<i32>,
    group: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MGrp {
    id: i32,
    name: String,
    gidx: i32,
    lib: Option<i32>,
}

async fn check_instance_state(db: &Db, insts: &[MInst], grps: &[MGrp], ctx: &str) {
    let mut got: Vec<MInst> = inst::get_all_instances(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| MInst {
            id: r.id,
            name: r.name,
            shortpath: r.shortpath,
            favorite: r.favorite,
            has_pack_update: r.has_pack_update,
            index: r.index,
            lib: r.library_position,
            group: r.group_id,
        })
        .collect();
    got.sort();
    let mut want = insts.to_vec();
    want.sort();
    assert_eq!(got, want, "instance rows diverged at {ctx}");

    let mut got_g: Vec<MGrp> = inst::get_all_groups(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| MGrp {
            id: r.id,
            name: r.name,
            gidx: r.group_index,
            lib: r.library_position,
        })
        .collect();
    got_g.sort();
    let mut want_g = grps.to_vec();
    want_g.sort();
    assert_eq!(got_g, want_g, "group rows diverged at {ctx}");
}

/// Upper bound for boundary-heavy parameter draws: one past the largest value
/// currently in play, so shifted values never drift out of the tested range.
fn hi_bound(values: impl Iterator<Item = i32>) -> i32 {
    values.max().unwrap_or(6).max(6) + 1
}

#[tokio::test]
async fn instance_reordering_matches_model() {
    for seed in [1u64, 2, 3] {
        let (_d, db) = migrated_db().await;
        let mut rng = Rng(seed);
        // Start the model from the migrated state, not from empty — the
        // migration chain is allowed to seed rows.
        let mut insts: Vec<MInst> = inst::get_all_instances(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| MInst {
                id: r.id,
                name: r.name,
                shortpath: r.shortpath,
                favorite: r.favorite,
                has_pack_update: r.has_pack_update,
                index: r.index,
                lib: r.library_position,
                group: r.group_id,
            })
            .collect();
        let mut grps: Vec<MGrp> = inst::get_all_groups(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| MGrp {
                id: r.id,
                name: r.name,
                gidx: r.group_index,
                lib: r.library_position,
            })
            .collect();

        for step in 0..400u32 {
            // Keep the population alive so ops always have targets.
            let forced_seed_group = grps.len() < 2;
            let forced_seed_instance = !forced_seed_group && insts.len() < 4;

            let op = if forced_seed_group {
                0
            } else if forced_seed_instance {
                1
            } else {
                rng.below(30)
            };
            let ctx = format!("seed {seed} step {step} op {op}");

            // Common parameter material.
            let lib_hi = hi_bound(insts.iter().filter_map(|r| r.lib));
            let idx_hi = hi_bound(insts.iter().map(|r| r.index));
            let glib_hi = hi_bound(grps.iter().filter_map(|g| g.lib));
            let some_group = if grps.is_empty() {
                999_999
            } else {
                grps[rng.below(grps.len() as u64) as usize].id
            };
            // Occasionally target a group id that does not exist.
            let scope_group = if rng.below(10) == 0 {
                999_999
            } else {
                some_group
            };
            let some_inst_id = if insts.is_empty() {
                999_999
            } else if rng.below(10) == 0 {
                999_999
            } else {
                insts[rng.below(insts.len() as u64) as usize].id
            };

            match op {
                // ---- population ----
                0 => {
                    let name = format!("g{}", rng.below(5));
                    let gidx = rng.int(0, 6);
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let id = inst::insert_group(&db, name.clone(), gidx, lib)
                        .await
                        .unwrap() as i32;
                    grps.push(MGrp {
                        id,
                        name,
                        gidx,
                        lib,
                    });
                }
                1 => {
                    let name = format!("n{}", rng.below(1000));
                    // Small shortpath pool so `delete_instances_by_shortpath`
                    // and the delete-then-insert of `add_instance_tx` hit
                    // multi-row cases.
                    let sp = format!("sp{}", rng.below(6));
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let id = inst::add_instance_tx(
                        &db,
                        name.clone(),
                        sp.clone(),
                        index,
                        some_group,
                        lib,
                    )
                    .await
                    .unwrap() as i32;
                    insts.retain(|r| r.shortpath != sp);
                    insts.push(MInst {
                        id,
                        name,
                        shortpath: sp,
                        favorite: false,
                        has_pack_update: false,
                        index,
                        lib,
                        group: some_group,
                    });
                }

                // ---- instance library-position shifts ----
                2 => {
                    let gt = rng.int(-1, lib_hi);
                    let lte = rng.int(-1, lib_hi);
                    let n = inst::shift_instance_library_positions_down(&db, gt, lte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if let Some(p) = r.lib {
                            if p > gt && p <= lte {
                                r.lib = Some(p - 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                3 => {
                    let gte = rng.int(-1, lib_hi);
                    let lt = rng.int(-1, lib_hi);
                    let n = inst::shift_instance_library_positions_up(&db, gte, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if let Some(p) = r.lib {
                            if p >= gte && p < lt {
                                r.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                4 => {
                    let gte = rng.int(-1, lib_hi);
                    let n = inst::shift_all_instance_library_positions_up(&db, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if let Some(p) = r.lib {
                            if p >= gte {
                                r.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                5 => {
                    let gt = rng.int(-1, lib_hi);
                    let n =
                        inst::shift_instance_library_positions_down_in_group(&db, scope_group, gt)
                            .await
                            .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group {
                            if let Some(p) = r.lib {
                                if p > gt {
                                    r.lib = Some(p - 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                6 => {
                    let gte = rng.int(-1, lib_hi);
                    let n = inst::shift_instance_library_positions_up_in_group_except(
                        &db,
                        scope_group,
                        gte,
                        some_inst_id,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group && r.id != some_inst_id {
                            if let Some(p) = r.lib {
                                if p >= gte {
                                    r.lib = Some(p + 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- instance index shifts ----
                7 => {
                    let gt = rng.int(-1, idx_hi);
                    let lt = rng.int(-1, idx_hi);
                    let n = inst::shift_instance_indexes_down_exclusive(&db, scope_group, gt, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group && r.index > gt && r.index < lt {
                            r.index -= 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                8 => {
                    let gte = rng.int(-1, idx_hi);
                    let lt = rng.int(-1, idx_hi);
                    let n = inst::shift_instance_indexes_up_range(&db, scope_group, gte, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group && r.index >= gte && r.index < lt {
                            r.index += 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                9 => {
                    let gt = rng.int(-1, idx_hi);
                    let n = inst::shift_instance_indexes_down_after(&db, scope_group, gt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group && r.index > gt {
                            r.index -= 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                10 => {
                    let gte = rng.int(-1, idx_hi);
                    let n = inst::shift_instance_indexes_up_from(&db, scope_group, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.group == scope_group && r.index >= gte {
                            r.index += 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- group library-position shifts ----
                11 => {
                    let gt = rng.int(-1, glib_hi);
                    let lte = rng.int(-1, glib_hi);
                    let n = inst::shift_group_library_positions_down(&db, gt, lte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p > gt && p <= lte {
                                g.lib = Some(p - 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                12 => {
                    let gte = rng.int(-1, glib_hi);
                    let lt = rng.int(-1, glib_hi);
                    let n = inst::shift_group_library_positions_up(&db, gte, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p >= gte && p < lt {
                                g.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                13 => {
                    let gte = rng.int(-1, glib_hi);
                    let n = inst::shift_all_group_library_positions_up_from(&db, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p >= gte {
                                g.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                14 => {
                    let gt = rng.int(-1, glib_hi);
                    let n = inst::shift_all_group_library_positions_down_after(&db, gt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p > gt {
                                g.lib = Some(p - 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- single-row updates ----
                15 => {
                    let fav = rng.flag();
                    let n = inst::set_instance_favorite(&db, some_inst_id, fav)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.id == some_inst_id {
                            r.favorite = fav;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                16 => {
                    let name = format!("rn{}", rng.below(1000));
                    // `Instance.shortpath` is UNIQUE; renames in the app always
                    // produce a fresh shortpath, so mirror that here.
                    let sp = format!("rsp{seed}_{step}");
                    let n = inst::set_instance_name_and_shortpath(&db, some_inst_id, &name, &sp)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.id == some_inst_id {
                            r.name = name.clone();
                            r.shortpath = sp.clone();
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                17 => {
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let n = inst::set_instance_index_and_library_position(
                        &db,
                        some_inst_id,
                        index,
                        lib,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.id == some_inst_id {
                            r.index = index;
                            r.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                18 => {
                    // FK: the target group must exist, like the managers guarantee.
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let n = inst::set_instance_group_index_library_position(
                        &db,
                        some_inst_id,
                        some_group,
                        index,
                        lib,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for r in insts.iter_mut() {
                        if r.id == some_inst_id {
                            r.group = some_group;
                            r.index = index;
                            r.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                19 => {
                    let gidx = rng.int(0, 8);
                    let n = inst::set_group_index(&db, some_group, gidx).await.unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.gidx = gidx;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                20 => {
                    let name = format!("g{}", rng.below(5));
                    let n = inst::set_group_name(&db, some_group, &name).await.unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.name = name.clone();
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                21 => {
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let n = inst::set_group_library_position(&db, some_group, lib)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                22 => {
                    let gidx = rng.int(0, 8);
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let n = inst::set_group_index_and_library_position(&db, some_group, gidx, lib)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.gidx = gidx;
                            g.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- deletes ----
                23 => {
                    // FKs are ON with `ON DELETE RESTRICT`, so only ever delete
                    // empty groups (as `delete_group_tx` guarantees by moving
                    // the members first). If none is empty, exercise the
                    // no-match path instead.
                    let empty: Vec<i32> = grps
                        .iter()
                        .filter(|g| !insts.iter().any(|r| r.group == g.id))
                        .map(|g| g.id)
                        .collect();
                    let target = if !empty.is_empty() && grps.len() > 2 {
                        empty[rng.below(empty.len() as u64) as usize]
                    } else {
                        999_999
                    };
                    let n = inst::delete_group(&db, target).await.unwrap();
                    let before = grps.len();
                    grps.retain(|g| g.id != target);
                    assert_eq!(n, before - grps.len(), "{ctx}");
                }
                24 => {
                    let sp = format!("sp{}", rng.below(6));
                    let n = inst::delete_instances_by_shortpath(&db, &sp).await.unwrap();
                    let before = insts.len();
                    insts.retain(|r| r.shortpath != sp);
                    assert_eq!(n, before - insts.len(), "{ctx}");
                }

                // ---- transaction helpers ----
                25 => {
                    // arrange_library_tx: restamp a random subset of groups and
                    // instances, mirroring what the drag&drop arrange does.
                    let mut garr = Vec::new();
                    for g in grps.iter_mut() {
                        if rng.flag() {
                            let gidx = rng.int(0, 8);
                            let lib = if rng.flag() {
                                Some(rng.int(0, glib_hi))
                            } else {
                                None
                            };
                            let set_lib = rng.flag();
                            garr.push(inst::GroupArrange {
                                id: g.id,
                                group_index: gidx,
                                library_position: lib,
                                set_library_position: set_lib,
                            });
                            g.gidx = gidx;
                            if set_lib {
                                g.lib = lib;
                            }
                        }
                    }
                    let mut iarr = Vec::new();
                    for r in insts.iter_mut() {
                        if rng.flag() {
                            let index = rng.int(0, idx_hi);
                            let lib = if rng.flag() {
                                Some(rng.int(0, lib_hi))
                            } else {
                                None
                            };
                            iarr.push(inst::InstanceArrange {
                                id: r.id,
                                index,
                                library_position: lib,
                            });
                            r.index = index;
                            r.lib = lib;
                        }
                    }
                    inst::arrange_library_tx(&db, garr, iarr).await.unwrap();
                }
                26 => {
                    // set_instance_indexes_tx: bulk index restamp.
                    let mut updates = Vec::new();
                    for r in insts.iter_mut() {
                        if rng.flag() {
                            let index = rng.int(0, idx_hi);
                            updates.push((r.id, index));
                            r.index = index;
                        }
                    }
                    inst::set_instance_indexes_tx(&db, updates).await.unwrap();
                }

                // ---- reads checked against the model ----
                27 => {
                    assert_eq!(
                        inst::count_groups(&db).await.unwrap(),
                        grps.len() as i64,
                        "{ctx}"
                    );
                    let name = format!("g{}", rng.below(5));
                    let got = inst::find_group_by_name(&db, &name).await.unwrap();
                    match got {
                        Some(row) => {
                            let m = grps
                                .iter()
                                .find(|g| g.id == row.id)
                                .unwrap_or_else(|| panic!("unknown group id {} at {ctx}", row.id));
                            assert_eq!(m.name, name, "{ctx}");
                        }
                        None => {
                            assert!(!grps.iter().any(|g| g.name == name), "{ctx}");
                        }
                    }
                }
                28 => {
                    let got = inst::first_instance_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    match got {
                        Some(row) => {
                            let m = insts.iter().find(|r| r.id == row.id).unwrap_or_else(|| {
                                panic!("unknown instance id {} at {ctx}", row.id)
                            });
                            assert_eq!(m.group, scope_group, "{ctx}");
                        }
                        None => {
                            assert!(!insts.iter().any(|r| r.group == scope_group), "{ctx}");
                        }
                    }
                    let got = inst::min_index_instance_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want = insts
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .map(|r| r.index)
                        .min();
                    assert_eq!(got.map(|r| r.index), want, "{ctx}");
                    let got_min = inst::min_library_position_instance_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want_min = insts
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .filter_map(|r| r.lib)
                        .min();
                    assert_eq!(got_min.and_then(|r| r.library_position), want_min, "{ctx}");
                    let got_max = inst::max_library_position_instance_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want_max = insts
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .filter_map(|r| r.lib)
                        .max();
                    assert_eq!(got_max.and_then(|r| r.library_position), want_max, "{ctx}");
                }
                _ => {
                    let got = inst::max_library_position_instance(&db).await.unwrap();
                    let want = insts.iter().filter_map(|r| r.lib).max();
                    assert_eq!(got.and_then(|r| r.library_position), want, "{ctx}");
                    let got = inst::max_library_position_group(&db).await.unwrap();
                    let want = grps.iter().filter_map(|g| g.lib).max();
                    assert_eq!(got.and_then(|g| g.library_position), want, "{ctx}");
                    let got = inst::min_library_position_group(&db).await.unwrap();
                    let want = grps.iter().filter_map(|g| g.lib).min();
                    assert_eq!(got.and_then(|g| g.library_position), want, "{ctx}");

                    // Ordered listings: same multiset (via the state check) and
                    // correctly ordered.
                    let rows = inst::get_all_instances_ordered_by_index(&db).await.unwrap();
                    assert_eq!(rows.len(), insts.len(), "{ctx}");
                    assert!(
                        rows.windows(2).all(|w| w[0].index <= w[1].index),
                        "index order violated at {ctx}"
                    );
                    let rows = inst::get_all_groups_ordered_by_group_index(&db)
                        .await
                        .unwrap();
                    assert_eq!(rows.len(), grps.len(), "{ctx}");
                    assert!(
                        rows.windows(2)
                            .all(|w| w[0].group_index <= w[1].group_index),
                        "group index order violated at {ctx}"
                    );
                    let rows = inst::get_groups_with_library_position_ordered(&db)
                        .await
                        .unwrap();
                    assert_eq!(
                        rows.len(),
                        grps.iter().filter(|g| g.lib.is_some()).count(),
                        "{ctx}"
                    );
                    assert!(
                        rows.windows(2)
                            .all(|w| w[0].library_position <= w[1].library_position),
                        "group library order violated at {ctx}"
                    );
                }
            }

            check_instance_state(&db, &insts, &grps, &ctx).await;
        }
    }
}

// ---------------------------------------------------------------------------
// Server family (mirrors the instance side, including the scoped variants
// that only exist on `Server`).
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct MSrv {
    id: i32,
    name: String,
    shortpath: String,
    favorite: bool,
    index: i32,
    lib: Option<i32>,
    group: i32,
    icon_revision: Option<i32>,
}

fn ts() -> DbDateTime {
    DbDateTime(from_millis(1_784_557_692_104).unwrap())
}

async fn check_server_state(db: &Db, servers: &[MSrv], grps: &[MGrp], ctx: &str) {
    let mut got: Vec<MSrv> = srv::get_all_servers(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| MSrv {
            id: r.id,
            name: r.name,
            shortpath: r.shortpath,
            favorite: r.favorite,
            index: r.index,
            lib: r.library_position,
            group: r.group_id,
            icon_revision: r.icon_revision,
        })
        .collect();
    got.sort();
    let mut want = servers.to_vec();
    want.sort();
    assert_eq!(got, want, "server rows diverged at {ctx}");

    let mut got_g: Vec<MGrp> = srv::get_all_server_groups(db)
        .await
        .unwrap()
        .into_iter()
        .map(|r| MGrp {
            id: r.id,
            name: r.name,
            gidx: r.group_index,
            lib: r.library_position,
        })
        .collect();
    got_g.sort();
    let mut want_g = grps.to_vec();
    want_g.sort();
    assert_eq!(got_g, want_g, "server group rows diverged at {ctx}");
}

#[tokio::test]
async fn server_reordering_matches_model() {
    for seed in [11u64, 12, 13] {
        let (_d, db) = migrated_db().await;
        let mut rng = Rng(seed);
        // Start the model from the migrated state — the `add_servers`
        // migration seeds a `Default` ServerGroup.
        let mut servers: Vec<MSrv> = srv::get_all_servers(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| MSrv {
                id: r.id,
                name: r.name,
                shortpath: r.shortpath,
                favorite: r.favorite,
                index: r.index,
                lib: r.library_position,
                group: r.group_id,
                icon_revision: r.icon_revision,
            })
            .collect();
        let mut grps: Vec<MGrp> = srv::get_all_server_groups(&db)
            .await
            .unwrap()
            .into_iter()
            .map(|r| MGrp {
                id: r.id,
                name: r.name,
                gidx: r.group_index,
                lib: r.library_position,
            })
            .collect();

        for step in 0..400u32 {
            let forced_seed_group = grps.len() < 2;
            let forced_seed_server = !forced_seed_group && servers.len() < 4;

            let op = if forced_seed_group {
                0
            } else if forced_seed_server {
                1
            } else {
                rng.below(28)
            };
            let ctx = format!("seed {seed} step {step} op {op}");

            let lib_hi = hi_bound(servers.iter().filter_map(|r| r.lib));
            let idx_hi = hi_bound(servers.iter().map(|r| r.index));
            let glib_hi = hi_bound(grps.iter().filter_map(|g| g.lib));
            let some_group = if grps.is_empty() {
                999_999
            } else {
                grps[rng.below(grps.len() as u64) as usize].id
            };
            let scope_group = if rng.below(10) == 0 {
                999_999
            } else {
                some_group
            };
            let some_srv_id = if servers.is_empty() {
                999_999
            } else if rng.below(10) == 0 {
                999_999
            } else {
                servers[rng.below(servers.len() as u64) as usize].id
            };

            match op {
                // ---- population ----
                0 => {
                    let name = format!("sg{}", rng.below(5));
                    let gidx = rng.int(0, 6);
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let id = srv::insert_server_group(&db, name.clone(), gidx, lib)
                        .await
                        .unwrap() as i32;
                    grps.push(MGrp {
                        id,
                        name,
                        gidx,
                        lib,
                    });
                }
                1 => {
                    let name = format!("sv{}", rng.below(1000));
                    // `Server.shortpath` is UNIQUE and `insert_server` has no
                    // delete-first, so shortpaths are step-unique.
                    let sp = format!("svp{seed}_{step}");
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let id = srv::insert_server(
                        &db,
                        name.clone(),
                        sp.clone(),
                        index,
                        some_group,
                        "1.20.1".into(),
                        25565,
                        "vanilla".into(),
                        None,
                        None,
                        None,
                        None,
                        None,
                        lib,
                        ts(),
                    )
                    .await
                    .unwrap() as i32;
                    servers.push(MSrv {
                        id,
                        name,
                        shortpath: sp,
                        favorite: false,
                        index,
                        lib,
                        group: some_group,
                        icon_revision: None,
                    });
                }

                // ---- server index shifts ----
                2 => {
                    let gt = rng.int(-1, idx_hi);
                    let lt = rng.int(-1, idx_hi);
                    let n = srv::shift_server_indexes_down_exclusive(&db, scope_group, gt, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group && r.index > gt && r.index < lt {
                            r.index -= 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                3 => {
                    let gte = rng.int(-1, idx_hi);
                    let lt = rng.int(-1, idx_hi);
                    let n = srv::shift_server_indexes_up_range(&db, scope_group, gte, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group && r.index >= gte && r.index < lt {
                            r.index += 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                4 => {
                    let gt = rng.int(-1, idx_hi);
                    let n = srv::shift_server_indexes_down_after(&db, scope_group, gt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group && r.index > gt {
                            r.index -= 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                5 => {
                    let gte = rng.int(-1, idx_hi);
                    let n = srv::shift_server_indexes_up_from(&db, scope_group, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group && r.index >= gte {
                            r.index += 1;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- server library-position shifts (all group-scoped) ----
                6 => {
                    let gt = rng.int(-1, lib_hi);
                    let n = srv::shift_server_library_positions_down_in_group(&db, scope_group, gt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group {
                            if let Some(p) = r.lib {
                                if p > gt {
                                    r.lib = Some(p - 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                7 => {
                    let gte = rng.int(-1, lib_hi);
                    let n = srv::shift_server_library_positions_up_in_group(&db, scope_group, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group {
                            if let Some(p) = r.lib {
                                if p >= gte {
                                    r.lib = Some(p + 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                8 => {
                    let gte = rng.int(-1, lib_hi);
                    let n = srv::shift_server_library_positions_up_in_group_except(
                        &db,
                        scope_group,
                        gte,
                        some_srv_id,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group && r.id != some_srv_id {
                            if let Some(p) = r.lib {
                                if p >= gte {
                                    r.lib = Some(p + 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                9 => {
                    let gt = rng.int(-1, lib_hi);
                    let lte = rng.int(-1, lib_hi);
                    let n =
                        srv::shift_server_library_positions_down_scoped(&db, scope_group, gt, lte)
                            .await
                            .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group {
                            if let Some(p) = r.lib {
                                if p > gt && p <= lte {
                                    r.lib = Some(p - 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                10 => {
                    let gte = rng.int(-1, lib_hi);
                    let lt = rng.int(-1, lib_hi);
                    let n =
                        srv::shift_server_library_positions_up_scoped(&db, scope_group, gte, lt)
                            .await
                            .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.group == scope_group {
                            if let Some(p) = r.lib {
                                if p >= gte && p < lt {
                                    r.lib = Some(p + 1);
                                    want += 1;
                                }
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- server group library-position shifts ----
                11 => {
                    let gt = rng.int(-1, glib_hi);
                    let lte = rng.int(-1, glib_hi);
                    let n = srv::shift_server_group_library_positions_down(&db, gt, lte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p > gt && p <= lte {
                                g.lib = Some(p - 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                12 => {
                    let gte = rng.int(-1, glib_hi);
                    let lt = rng.int(-1, glib_hi);
                    let n = srv::shift_server_group_library_positions_up(&db, gte, lt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p >= gte && p < lt {
                                g.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                13 => {
                    let gte = rng.int(-1, glib_hi);
                    let n = srv::shift_all_server_group_library_positions_up_from(&db, gte)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p >= gte {
                                g.lib = Some(p + 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                14 => {
                    let gt = rng.int(-1, glib_hi);
                    let n = srv::shift_all_server_group_library_positions_down_after(&db, gt)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if let Some(p) = g.lib {
                            if p > gt {
                                g.lib = Some(p - 1);
                                want += 1;
                            }
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- single-row updates ----
                15 => {
                    let fav = rng.flag();
                    let n = srv::set_server_favorite(&db, some_srv_id, fav)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.id == some_srv_id {
                            r.favorite = fav;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                16 => {
                    let rev = if rng.flag() {
                        Some(rng.int(0, 9))
                    } else {
                        None
                    };
                    let n = srv::set_server_icon_revision(&db, some_srv_id, rev)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.id == some_srv_id {
                            r.icon_revision = rev;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                17 => {
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let n =
                        srv::set_server_index_and_library_position(&db, some_srv_id, index, lib)
                            .await
                            .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.id == some_srv_id {
                            r.index = index;
                            r.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                18 => {
                    let index = rng.int(0, idx_hi);
                    let lib = if rng.flag() {
                        Some(rng.int(0, lib_hi))
                    } else {
                        None
                    };
                    let n = srv::set_server_group_index_library_position(
                        &db,
                        some_srv_id,
                        some_group,
                        index,
                        lib,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for r in servers.iter_mut() {
                        if r.id == some_srv_id {
                            r.group = some_group;
                            r.index = index;
                            r.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                19 => {
                    let gidx = rng.int(0, 8);
                    let n = srv::set_server_group_index(&db, some_group, gidx)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.gidx = gidx;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                20 => {
                    let name = format!("sg{}", rng.below(5));
                    let n = srv::set_server_group_name(&db, some_group, &name)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.name = name.clone();
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                21 => {
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let n = srv::set_server_group_library_position(&db, some_group, lib)
                        .await
                        .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }
                22 => {
                    let gidx = rng.int(0, 8);
                    let lib = if rng.flag() {
                        Some(rng.int(0, glib_hi))
                    } else {
                        None
                    };
                    let n = srv::set_server_group_index_and_library_position(
                        &db, some_group, gidx, lib,
                    )
                    .await
                    .unwrap();
                    let mut want = 0;
                    for g in grps.iter_mut() {
                        if g.id == some_group {
                            g.gidx = gidx;
                            g.lib = lib;
                            want += 1;
                        }
                    }
                    assert_eq!(n, want, "{ctx}");
                }

                // ---- deletes ----
                23 => {
                    let n = srv::delete_server(&db, some_srv_id).await.unwrap();
                    let before = servers.len();
                    servers.retain(|r| r.id != some_srv_id);
                    assert_eq!(n, before - servers.len(), "{ctx}");
                }
                24 => {
                    let empty: Vec<i32> = grps
                        .iter()
                        .filter(|g| !servers.iter().any(|r| r.group == g.id))
                        .map(|g| g.id)
                        .collect();
                    let target = if !empty.is_empty() && grps.len() > 2 {
                        empty[rng.below(empty.len() as u64) as usize]
                    } else {
                        999_999
                    };
                    let n = srv::delete_server_group(&db, target).await.unwrap();
                    let before = grps.len();
                    grps.retain(|g| g.id != target);
                    assert_eq!(n, before - grps.len(), "{ctx}");
                }

                // ---- reads checked against the model ----
                25 => {
                    assert_eq!(
                        srv::count_server_groups(&db).await.unwrap(),
                        grps.len() as i64,
                        "{ctx}"
                    );
                    let name = format!("sg{}", rng.below(5));
                    let got = srv::find_server_group_by_name(&db, &name).await.unwrap();
                    match got {
                        Some(row) => {
                            let m = grps.iter().find(|g| g.id == row.id).unwrap_or_else(|| {
                                panic!("unknown server group id {} at {ctx}", row.id)
                            });
                            assert_eq!(m.name, name, "{ctx}");
                        }
                        None => {
                            assert!(!grps.iter().any(|g| g.name == name), "{ctx}");
                        }
                    }
                }
                26 => {
                    let got = srv::first_server_in_group(&db, scope_group).await.unwrap();
                    match got {
                        Some(row) => {
                            let m = servers
                                .iter()
                                .find(|r| r.id == row.id)
                                .unwrap_or_else(|| panic!("unknown server id {} at {ctx}", row.id));
                            assert_eq!(m.group, scope_group, "{ctx}");
                        }
                        None => {
                            assert!(!servers.iter().any(|r| r.group == scope_group), "{ctx}");
                        }
                    }
                    let got = srv::min_index_server_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want = servers
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .map(|r| r.index)
                        .min();
                    assert_eq!(got.map(|r| r.index), want, "{ctx}");
                    let got = srv::min_library_position_server_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want = servers
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .filter_map(|r| r.lib)
                        .min();
                    assert_eq!(got.and_then(|r| r.library_position), want, "{ctx}");
                    let got = srv::max_library_position_server_in_group(&db, scope_group)
                        .await
                        .unwrap();
                    let want = servers
                        .iter()
                        .filter(|r| r.group == scope_group)
                        .filter_map(|r| r.lib)
                        .max();
                    assert_eq!(got.and_then(|r| r.library_position), want, "{ctx}");
                }
                _ => {
                    let got = srv::max_library_position_server_group(&db).await.unwrap();
                    let want = grps.iter().filter_map(|g| g.lib).max();
                    assert_eq!(got.and_then(|g| g.library_position), want, "{ctx}");
                    let got = srv::min_library_position_server_group(&db).await.unwrap();
                    let want = grps.iter().filter_map(|g| g.lib).min();
                    assert_eq!(got.and_then(|g| g.library_position), want, "{ctx}");

                    let rows = srv::get_all_servers_ordered_by_index(&db).await.unwrap();
                    assert_eq!(rows.len(), servers.len(), "{ctx}");
                    assert!(
                        rows.windows(2).all(|w| w[0].index <= w[1].index),
                        "server index order violated at {ctx}"
                    );
                    let rows = srv::get_all_server_groups_ordered_by_group_index(&db)
                        .await
                        .unwrap();
                    assert_eq!(rows.len(), grps.len(), "{ctx}");
                    assert!(
                        rows.windows(2)
                            .all(|w| w[0].group_index <= w[1].group_index),
                        "server group index order violated at {ctx}"
                    );
                    let rows = srv::get_server_groups_with_library_position_ordered(&db)
                        .await
                        .unwrap();
                    assert_eq!(
                        rows.len(),
                        grps.iter().filter(|g| g.lib.is_some()).count(),
                        "{ctx}"
                    );
                    assert!(
                        rows.windows(2)
                            .all(|w| w[0].library_position <= w[1].library_position),
                        "server group library order violated at {ctx}"
                    );
                }
            }

            check_server_state(&db, &servers, &grps, &ctx).await;
        }
    }
}
