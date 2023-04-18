#[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
use solana_rbpf::error::EbpfError;
use {
    crate::{invoke_context::InvokeContext, timings::ExecuteDetailsTimings},
    itertools::Itertools,
    solana_measure::measure::Measure,
    solana_rbpf::{
        elf::Executable,
        verifier::RequisiteVerifier,
        vm::{BuiltInProgram, VerifiedExecutable},
    },
    solana_sdk::{
        bpf_loader, bpf_loader_deprecated, bpf_loader_upgradeable, clock::Slot, loader_v3,
        pubkey::Pubkey, saturating_add_assign,
    },
    std::{
        collections::HashMap,
        fmt::{Debug, Formatter},
        sync::{
            atomic::{AtomicU64, Ordering},
            Arc,
        },
    },
};

const MAX_CACHE_ENTRIES: usize = 100; // TODO: Tune to size

/// Relationship between two fork IDs
#[derive(Copy, Clone, PartialEq)]
pub enum BlockRelation {
    /// The slot is on the same fork and is an ancestor of the other slot
    Ancestor,
    /// The two slots are equal and are on the same fork
    Equal,
    /// The slot is on the same fork and is a descendant of the other slot
    Descendant,
    /// The slots are on two different forks and may have had a common ancestor at some point
    Unrelated,
    /// Either one or both of the slots are either older than the latest root, or are in future
    Unknown,
}

/// Maps relationship between two slots.
pub trait ForkGraph {
    /// Returns the BlockRelation of A to B
    fn relationship(&self, a: Slot, b: Slot) -> BlockRelation;
}

/// Provides information about current working slot, and its ancestors
pub trait WorkingSlot {
    /// Returns the current slot value
    fn current_slot(&self) -> Slot;

    /// Returns true if the `other` slot is an ancestor of self, false otherwise
    fn is_ancestor(&self, other: Slot) -> bool;
}

#[derive(Default)]
pub enum LoadedProgramType {
    /// Tombstone for undeployed, closed or unloadable programs
    #[default]
    FailedVerification,
    Closed,
    DelayVisibility,
    LegacyV0(VerifiedExecutable<RequisiteVerifier, InvokeContext<'static>>),
    LegacyV1(VerifiedExecutable<RequisiteVerifier, InvokeContext<'static>>),
    Typed(VerifiedExecutable<RequisiteVerifier, InvokeContext<'static>>),
    BuiltIn(BuiltInProgram<InvokeContext<'static>>),
}

impl Debug for LoadedProgramType {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadedProgramType::FailedVerification => {
                write!(f, "LoadedProgramType::FailedVerification")
            }
            LoadedProgramType::Closed => write!(f, "LoadedProgramType::Closed"),
            LoadedProgramType::DelayVisibility => write!(f, "LoadedProgramType::DelayVisibility"),
            LoadedProgramType::LegacyV0(_) => write!(f, "LoadedProgramType::LegacyV0"),
            LoadedProgramType::LegacyV1(_) => write!(f, "LoadedProgramType::LegacyV1"),
            LoadedProgramType::Typed(_) => write!(f, "LoadedProgramType::Typed"),
            LoadedProgramType::BuiltIn(_) => write!(f, "LoadedProgramType::BuiltIn"),
        }
    }
}

#[derive(Debug, Default)]
pub struct LoadedProgram {
    /// The program of this entry
    pub program: LoadedProgramType,
    /// Size of account that stores the program and program data
    pub account_size: usize,
    /// Slot in which the program was (re)deployed
    pub deployment_slot: Slot,
    /// Slot in which this entry will become active (can be in the future)
    pub effective_slot: Slot,
    /// Optional expiration slot for this entry, after which it is treated as non-existent
    pub maybe_expiration_slot: Option<Slot>,
    /// How often this entry was used
    pub usage_counter: AtomicU64,
}

#[derive(Debug, Default)]
pub struct LoadProgramMetrics {
    pub program_id: String,
    pub register_syscalls_us: u64,
    pub load_elf_us: u64,
    pub verify_code_us: u64,
    pub jit_compile_us: u64,
}

impl LoadProgramMetrics {
    pub fn submit_datapoint(&self, timings: &mut ExecuteDetailsTimings) {
        saturating_add_assign!(
            timings.create_executor_register_syscalls_us,
            self.register_syscalls_us
        );
        saturating_add_assign!(timings.create_executor_load_elf_us, self.load_elf_us);
        saturating_add_assign!(timings.create_executor_verify_code_us, self.verify_code_us);
        saturating_add_assign!(timings.create_executor_jit_compile_us, self.jit_compile_us);
        datapoint_trace!(
            "create_executor_trace",
            ("program_id", self.program_id, String),
            ("register_syscalls_us", self.register_syscalls_us, i64),
            ("load_elf_us", self.load_elf_us, i64),
            ("verify_code_us", self.verify_code_us, i64),
            ("jit_compile_us", self.jit_compile_us, i64),
        );
    }
}

impl LoadedProgram {
    /// Creates a new user program
    pub fn new(
        loader_key: &Pubkey,
        loader: Arc<BuiltInProgram<InvokeContext<'static>>>,
        deployment_slot: Slot,
        effective_slot: Slot,
        maybe_expiration_slot: Option<Slot>,
        elf_bytes: &[u8],
        account_size: usize,
        use_jit: bool,
        metrics: &mut LoadProgramMetrics,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let mut load_elf_time = Measure::start("load_elf_time");
        let executable = Executable::load(elf_bytes, loader.clone())?;
        load_elf_time.stop();
        metrics.load_elf_us = load_elf_time.as_us();

        let mut verify_code_time = Measure::start("verify_code_time");

        // Allowing mut here, since it may be needed for jit compile, which is under a config flag
        #[allow(unused_mut)]
        let mut program = if bpf_loader_deprecated::check_id(loader_key) {
            LoadedProgramType::LegacyV0(VerifiedExecutable::from_executable(executable)?)
        } else if bpf_loader::check_id(loader_key) || bpf_loader_upgradeable::check_id(loader_key) {
            LoadedProgramType::LegacyV1(VerifiedExecutable::from_executable(executable)?)
        } else if loader_v3::check_id(loader_key) {
            LoadedProgramType::Typed(VerifiedExecutable::from_executable(executable)?)
        } else {
            panic!();
        };
        verify_code_time.stop();
        metrics.verify_code_us = verify_code_time.as_us();

        if use_jit {
            #[cfg(all(not(target_os = "windows"), target_arch = "x86_64"))]
            {
                let mut jit_compile_time = Measure::start("jit_compile_time");
                match &mut program {
                    LoadedProgramType::LegacyV0(executable) => executable.jit_compile(),
                    LoadedProgramType::LegacyV1(executable) => executable.jit_compile(),
                    LoadedProgramType::Typed(executable) => executable.jit_compile(),
                    _ => Err(EbpfError::JitNotCompiled),
                }?;
                jit_compile_time.stop();
                metrics.jit_compile_us = jit_compile_time.as_us();
            }
        }

        Ok(Self {
            deployment_slot,
            account_size,
            effective_slot,
            maybe_expiration_slot,
            usage_counter: AtomicU64::new(0),
            program,
        })
    }

    /// Creates a new built-in program
    pub fn new_built_in(
        deployment_slot: Slot,
        program: BuiltInProgram<InvokeContext<'static>>,
    ) -> Self {
        Self {
            deployment_slot,
            account_size: 0,
            effective_slot: deployment_slot.saturating_add(1),
            maybe_expiration_slot: None,
            usage_counter: AtomicU64::new(0),
            program: LoadedProgramType::BuiltIn(program),
        }
    }

    pub fn new_tombstone(slot: Slot, reason: LoadedProgramType) -> Self {
        let maybe_expiration_slot =
            matches!(reason, LoadedProgramType::DelayVisibility).then_some(slot.saturating_add(1));
        let tombstone = Self {
            program: reason,
            account_size: 0,
            deployment_slot: slot,
            effective_slot: slot,
            maybe_expiration_slot,
            usage_counter: AtomicU64::default(),
        };
        debug_assert!(tombstone.is_tombstone());
        tombstone
    }

    pub fn is_tombstone(&self) -> bool {
        matches!(
            self.program,
            LoadedProgramType::FailedVerification
                | LoadedProgramType::Closed
                | LoadedProgramType::DelayVisibility
        )
    }
}

#[derive(Debug, Default)]
pub struct LoadedPrograms {
    /// A two level index:
    ///
    /// Pubkey is the address of a program, multiple versions can coexists simultaneously under the same address (in different slots).
    entries: HashMap<Pubkey, Vec<Arc<LoadedProgram>>>,
}

#[cfg(RUSTC_WITH_SPECIALIZATION)]
impl solana_frozen_abi::abi_example::AbiExample for LoadedPrograms {
    fn example() -> Self {
        // Delegate AbiExample impl to Default before going deep and stuck with
        // not easily impl-able Arc<dyn Executor> due to rust's coherence issue
        // This is safe because LoadedPrograms isn't serializable by definition.
        Self::default()
    }
}

impl LoadedPrograms {
    /// Refill the cache with a single entry. It's typically called during transaction loading,
    /// when the cache doesn't contain the entry corresponding to program `key`.
    /// The function dedupes the cache, in case some other thread replenished the entry in parallel.
    pub fn replenish(
        &mut self,
        key: Pubkey,
        entry: Arc<LoadedProgram>,
    ) -> (bool, Arc<LoadedProgram>) {
        let second_level = self.entries.entry(key).or_insert_with(Vec::new);
        let index = second_level
            .iter()
            .position(|at| at.effective_slot >= entry.effective_slot);
        if let Some(existing) = index.and_then(|index| second_level.get(index)) {
            if existing.deployment_slot == entry.deployment_slot
                && existing.effective_slot == entry.effective_slot
            {
                return (true, existing.clone());
            }
        }
        second_level.insert(index.unwrap_or(second_level.len()), entry.clone());
        (false, entry)
    }

    /// Assign the program `entry` to the given `key` in the cache.
    /// This is typically called when a deployed program is managed (un-/re-/deployed) via
    /// loader instructions. Because of the cooldown, entires can not have the same
    /// deployment_slot and effective_slot.
    pub fn assign_program(&mut self, key: Pubkey, entry: Arc<LoadedProgram>) -> Arc<LoadedProgram> {
        let (was_occupied, entry) = self.replenish(key, entry);
        debug_assert!(!was_occupied);
        entry
    }

    /// Before rerooting the blockstore this removes all programs of orphan forks
    pub fn prune<F: ForkGraph>(&mut self, fork_graph: &F, new_root: Slot) {
        self.entries.retain(|_key, second_level| {
            let mut first_ancestor = true;
            *second_level = second_level
                .iter()
                .rev()
                .filter(|entry| {
                    let relation = fork_graph.relationship(entry.deployment_slot, new_root);
                    if entry.deployment_slot >= new_root {
                        matches!(relation, BlockRelation::Equal | BlockRelation::Descendant)
                    } else if first_ancestor {
                        first_ancestor = false;
                        matches!(relation, BlockRelation::Ancestor)
                    } else {
                        false
                    }
                })
                .cloned()
                .collect();
            second_level.reverse();
            !second_level.is_empty()
        });
    }

    /// Extracts a subset of the programs relevant to a transaction batch
    /// and returns which program accounts the accounts DB needs to load.
    pub fn extract<S: WorkingSlot>(
        &self,
        working_slot: &S,
        keys: impl Iterator<Item = Pubkey>,
    ) -> (HashMap<Pubkey, Arc<LoadedProgram>>, Vec<Pubkey>) {
        let mut missing = Vec::new();
        let found = keys
            .filter_map(|key| {
                if let Some(second_level) = self.entries.get(&key) {
                    for entry in second_level.iter().rev() {
                        let current_slot = working_slot.current_slot();
                        if current_slot == entry.deployment_slot
                            || working_slot.is_ancestor(entry.deployment_slot)
                        {
                            if entry
                                .maybe_expiration_slot
                                .map(|expiration_slot| current_slot >= expiration_slot)
                                .unwrap_or(false)
                            {
                                // Found an entry that's already expired. Any further entries in the list
                                // are older than the current one. So treat the program as missing in the
                                // cache and return early.
                                missing.push(key);
                                return None;
                            }

                            if current_slot >= entry.effective_slot {
                                return Some((key, entry.clone()));
                            }
                        }
                    }
                }
                missing.push(key);
                None
            })
            .collect();
        (found, missing)
    }

    /// Evicts programs which were used infrequently
    pub fn sort_and_evict(&mut self, max_cache_entries: Option<usize>) {
        // Find eviction candidates and sort by their usage counters
        let mut num_cache_entries: usize = 0;
        let sorted_candidates = self
            .entries
            .iter()
            .filter(|(_key, programs)| {
                num_cache_entries = num_cache_entries.saturating_add(programs.len());
                programs.len() == 1
            })
            .sorted_by_cached_key(|(_key, programs)| {
                programs
                    .get(0)
                    .unwrap()
                    .usage_counter
                    .load(Ordering::Relaxed)
            })
            .map(|(key, _programs)| *key)
            .collect::<Vec<Pubkey>>();
        // Calculate how many to remove
        let num_to_remove = std::cmp::min(
            num_cache_entries.saturating_sub(max_cache_entries.unwrap_or(MAX_CACHE_ENTRIES)),
            sorted_candidates.len(),
        );
        // Remove selected entries
        if num_to_remove != 0 {
            self.remove_entries(sorted_candidates.into_iter().take(num_to_remove))
        }
    }

    /// Removes the entries at the given keys, if they exist
    pub fn remove_entries(&mut self, keys: impl Iterator<Item = Pubkey>) {
        for k in keys {
            self.entries.remove(&k);
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        crate::loaded_programs::{
            BlockRelation, ForkGraph, LoadedProgram, LoadedProgramType, LoadedPrograms, WorkingSlot,
        },
        solana_rbpf::vm::BuiltInProgram,
        solana_sdk::{clock::Slot, pubkey::Pubkey},
        std::{
            collections::HashMap,
            ops::ControlFlow,
            sync::{
                atomic::{AtomicU64, Ordering},
                Arc,
            },
        },
    };

    fn new_test_builtin_program(deployment_slot: Slot, effective_slot: Slot) -> Arc<LoadedProgram> {
        Arc::new(LoadedProgram {
            program: LoadedProgramType::BuiltIn(BuiltInProgram::default()),
            account_size: 0,
            deployment_slot,
            effective_slot,
            maybe_expiration_slot: None,
            usage_counter: AtomicU64::default(),
        })
    }

    fn set_tombstone(cache: &mut LoadedPrograms, key: Pubkey, slot: Slot) -> Arc<LoadedProgram> {
        cache.assign_program(
            key,
            Arc::new(LoadedProgram::new_tombstone(
                slot,
                LoadedProgramType::FailedVerification,
            )),
        )
    }

    #[test]
    fn test_eviction() {
        // Fork graph created for the test
        //                   0
        //                 /   \
        //                10    5
        //                |     |
        //                20    11
        //                |     | \
        //                22   15  25
        //                      |   |
        //                     16  27
        let mut fork_graph = TestForkGraphSpecific::default();
        fork_graph.insert_fork(&[0, 10, 20, 22]);
        fork_graph.insert_fork(&[0, 5, 11, 15, 16]);
        fork_graph.insert_fork(&[0, 5, 11, 25, 27]);
        let possible_slots: Vec<u64> = vec![0, 5, 10, 11, 15, 16, 20, 22, 25, 27];
        let usage_counters: Vec<u64> = vec![43, 10, 1128, 1, 0, 67, 212, 322, 29, 21];
        let mut programs = HashMap::<Pubkey, Vec<(u64, u64)>>::new();
        let mut num_total_programs: usize = 0;

        let mut cache = LoadedPrograms::default();

        let program1 = Pubkey::new_unique();
        let program1_deployment_slots = vec![0, 10, 20];
        let program1_usage_counters = vec![1, 5, 25];
        program1_deployment_slots
            .iter()
            .enumerate()
            .for_each(|(i, deployment_slot)| {
                cache.replenish(
                    program1,
                    new_test_loaded_program_with_usage(
                        *deployment_slot,
                        (*deployment_slot) + 2,
                        AtomicU64::new(*program1_usage_counters.get(i).unwrap_or(&0)),
                    ),
                );
                num_total_programs += 1;
                programs
                    .entry(program1)
                    .and_modify(|entries| {
                        entries.push((
                            *deployment_slot,
                            *program1_usage_counters.get(i).unwrap_or(&0),
                        ))
                    })
                    .or_insert_with(|| {
                        Vec::<(u64, u64)>::from([(
                            *deployment_slot,
                            *program1_usage_counters.get(i).unwrap_or(&0),
                        )])
                    });
            });

        let program2 = Pubkey::new_unique();
        let program2_deployment_slots = vec![5, 11];
        let program2_usage_counters = vec![0, 10];
        program2_deployment_slots
            .iter()
            .enumerate()
            .for_each(|(i, deployment_slot)| {
                cache.replenish(
                    program2,
                    new_test_loaded_program_with_usage(
                        *deployment_slot,
                        (*deployment_slot) + 2,
                        AtomicU64::new(*program2_usage_counters.get(i).unwrap_or(&0)),
                    ),
                );
                num_total_programs += 1;
                programs
                    .entry(program2)
                    .and_modify(|entries| {
                        entries.push((
                            *deployment_slot,
                            *program2_usage_counters.get(i).unwrap_or(&0),
                        ))
                    })
                    .or_insert_with(|| {
                        Vec::<(u64, u64)>::from([(
                            *deployment_slot,
                            *program2_usage_counters.get(i).unwrap_or(&0),
                        )])
                    });
            });

        let program3 = Pubkey::new_unique();
        let program3_deployment_slots = vec![0, 5, 15];
        let program3_usage_counters = vec![100, 3, 20];
        program3_deployment_slots
            .iter()
            .enumerate()
            .for_each(|(i, deployment_slot)| {
                cache.replenish(
                    program3,
                    new_test_loaded_program_with_usage(
                        *deployment_slot,
                        (*deployment_slot) + 2,
                        AtomicU64::new(*program3_usage_counters.get(i).unwrap_or(&0)),
                    ),
                );
                num_total_programs += 1;
                programs
                    .entry(program3)
                    .and_modify(|entries| {
                        entries.push((
                            *deployment_slot,
                            *program3_usage_counters.get(i).unwrap_or(&0),
                        ))
                    })
                    .or_insert_with(|| {
                        Vec::<(u64, u64)>::from([(
                            *deployment_slot,
                            *program3_usage_counters.get(i).unwrap_or(&0),
                        )])
                    });
            });

        // Add random set of used programs (with no redeploys) on each possible slot
        // in the fork graph
        let mut eviction_candidates = possible_slots
            .into_iter()
            .enumerate()
            .map(|(i, slot)| {
                (
                    Pubkey::new_unique(),
                    slot,
                    *usage_counters.get(i).unwrap_or(&0),
                )
            })
            .collect::<Vec<_>>();
        eviction_candidates
            .iter()
            .for_each(|(key, deployment_slot, usage_counter)| {
                cache.replenish(
                    *key,
                    new_test_loaded_program_with_usage(
                        *deployment_slot,
                        (*deployment_slot) + 2,
                        AtomicU64::new(*usage_counter),
                    ),
                );
                num_total_programs += 1;
                programs
                    .entry(*key)
                    .and_modify(|entries| entries.push((*deployment_slot, *usage_counter)))
                    .or_insert_with(|| {
                        Vec::<(u64, u64)>::from([(*deployment_slot, *usage_counter)])
                    });
            });
        eviction_candidates.sort_by_key(|(_key, _deplyment_slot, usage_counter)| *usage_counter);

        // Try to remove no programs.
        cache.sort_and_evict(Some(num_total_programs));
        // Check that every program is still in the cache.
        programs.iter().for_each(|entry| {
            assert!(cache.entries.get(entry.0).is_some());
        });

        // Try to remove less than max programs.
        let max_cache_entries = 12_usize;
        // Guarantee you won't evict all eviction candidates
        let num_to_remove = num_total_programs - max_cache_entries;
        assert!(eviction_candidates.len() > num_to_remove);
        let removals = eviction_candidates
            .drain(0..num_to_remove)
            .map(|(key, _, _)| key)
            .collect::<Vec<_>>();
        cache.sort_and_evict(Some(max_cache_entries));
        // Make sure removed entries are gone
        removals.iter().for_each(|key| {
            assert!(cache.entries.get(key).is_none());
        });
        // Make sure the other entries are still present in the cache
        programs
            .iter()
            .filter(|(key, _)| !removals.contains(key))
            .for_each(
                // For every entry not removed
                |(key, val)| {
                    let program_in_cache = cache.entries.get(key);
                    assert!(program_in_cache.is_some()); // Make sure it's entry exists
                    let values_in_cache = program_in_cache
                        .unwrap()
                        .iter()
                        .map(|x| (x.deployment_slot, x.usage_counter.load(Ordering::Relaxed)))
                        .collect::<Vec<_>>();
                    val.iter().for_each(|entry| {
                        // make sure the exact slot and usage counter remain
                        // for the entry
                        assert!(values_in_cache.contains(entry));
                    });
                },
            );
        // Remove entries from you local cache tracker
        removals.iter().for_each(|key| {
            programs.remove(key);
            num_total_programs -= 1;
        });

        // Try to remove all programs.
        let max_num_removals = eviction_candidates.len();
        // Make sure total programs is greater than number of eviction candidates
        assert!(num_total_programs > max_num_removals);
        cache.sort_and_evict(Some(0));
        // Make sure all candidate removals were removed
        let removals = eviction_candidates
            .iter()
            .map(|(key, _, _)| key)
            .collect::<Vec<_>>();
        removals.iter().for_each(|key| {
            assert!(cache.entries.get(*key).is_none());
        });
        // Make sure all non-candidate removals remain
        programs
            .iter()
            .filter(|(key, _)| !removals.contains(key))
            .for_each(
                // For every entry not removed
                |(key, val)| {
                    let program_in_cache = cache.entries.get(key);
                    assert!(program_in_cache.is_some()); // Make sure it's entry exists
                    let values_in_cache = program_in_cache
                        .unwrap()
                        .iter()
                        .map(|x| (x.deployment_slot, x.usage_counter.load(Ordering::Relaxed)))
                        .collect::<Vec<_>>();
                    val.iter().for_each(|entry| {
                        // make sure the exact slot and usage counter remain
                        // for the entry
                        assert!(values_in_cache.contains(entry));
                    });
                },
            );
    }

    #[test]
    fn test_tombstone() {
        let tombstone = LoadedProgram::new_tombstone(0, LoadedProgramType::FailedVerification);
        assert!(matches!(
            tombstone.program,
            LoadedProgramType::FailedVerification
        ));
        assert!(tombstone.is_tombstone());
        assert_eq!(tombstone.deployment_slot, 0);
        assert_eq!(tombstone.effective_slot, 0);

        let tombstone = LoadedProgram::new_tombstone(100, LoadedProgramType::Closed);
        assert!(matches!(tombstone.program, LoadedProgramType::Closed));
        assert!(tombstone.is_tombstone());
        assert_eq!(tombstone.deployment_slot, 100);
        assert_eq!(tombstone.effective_slot, 100);

        let mut cache = LoadedPrograms::default();
        let program1 = Pubkey::new_unique();
        let tombstone = set_tombstone(&mut cache, program1, 10);
        let second_level = &cache
            .entries
            .get(&program1)
            .expect("Failed to find the entry");
        assert_eq!(second_level.len(), 1);
        assert!(second_level.get(0).unwrap().is_tombstone());
        assert_eq!(tombstone.deployment_slot, 10);
        assert_eq!(tombstone.effective_slot, 10);

        // Add a program at slot 50, and a tombstone for the program at slot 60
        let program2 = Pubkey::new_unique();
        assert!(
            !cache
                .replenish(program2, new_test_builtin_program(50, 51))
                .0
        );
        let second_level = &cache
            .entries
            .get(&program2)
            .expect("Failed to find the entry");
        assert_eq!(second_level.len(), 1);
        assert!(!second_level.get(0).unwrap().is_tombstone());

        let tombstone = set_tombstone(&mut cache, program2, 60);
        let second_level = &cache
            .entries
            .get(&program2)
            .expect("Failed to find the entry");
        assert_eq!(second_level.len(), 2);
        assert!(!second_level.get(0).unwrap().is_tombstone());
        assert!(second_level.get(1).unwrap().is_tombstone());
        assert!(tombstone.is_tombstone());
        assert_eq!(tombstone.deployment_slot, 60);
        assert_eq!(tombstone.effective_slot, 60);
    }

    struct TestForkGraph {
        relation: BlockRelation,
    }
    impl ForkGraph for TestForkGraph {
        fn relationship(&self, _a: Slot, _b: Slot) -> BlockRelation {
            self.relation
        }
    }

    #[test]
    fn test_prune_empty() {
        let mut cache = LoadedPrograms::default();
        let fork_graph = TestForkGraph {
            relation: BlockRelation::Unrelated,
        };

        cache.prune(&fork_graph, 0);
        assert!(cache.entries.is_empty());

        cache.prune(&fork_graph, 10);
        assert!(cache.entries.is_empty());

        let fork_graph = TestForkGraph {
            relation: BlockRelation::Ancestor,
        };

        cache.prune(&fork_graph, 0);
        assert!(cache.entries.is_empty());

        cache.prune(&fork_graph, 10);
        assert!(cache.entries.is_empty());

        let fork_graph = TestForkGraph {
            relation: BlockRelation::Descendant,
        };

        cache.prune(&fork_graph, 0);
        assert!(cache.entries.is_empty());

        cache.prune(&fork_graph, 10);
        assert!(cache.entries.is_empty());

        let fork_graph = TestForkGraph {
            relation: BlockRelation::Unknown,
        };

        cache.prune(&fork_graph, 0);
        assert!(cache.entries.is_empty());

        cache.prune(&fork_graph, 10);
        assert!(cache.entries.is_empty());
    }

    #[derive(Default)]
    struct TestForkGraphSpecific {
        forks: Vec<Vec<Slot>>,
    }

    impl TestForkGraphSpecific {
        fn insert_fork(&mut self, fork: &[Slot]) {
            let mut fork = fork.to_vec();
            fork.sort();
            self.forks.push(fork)
        }
    }

    impl ForkGraph for TestForkGraphSpecific {
        fn relationship(&self, a: Slot, b: Slot) -> BlockRelation {
            match self.forks.iter().try_for_each(|fork| {
                let relation = fork
                    .iter()
                    .position(|x| *x == a)
                    .and_then(|a_pos| {
                        fork.iter().position(|x| *x == b).and_then(|b_pos| {
                            (a_pos == b_pos)
                                .then_some(BlockRelation::Equal)
                                .or_else(|| (a_pos < b_pos).then_some(BlockRelation::Ancestor))
                                .or(Some(BlockRelation::Descendant))
                        })
                    })
                    .unwrap_or(BlockRelation::Unrelated);

                if relation != BlockRelation::Unrelated {
                    return ControlFlow::Break(relation);
                }

                ControlFlow::Continue(())
            }) {
                ControlFlow::Break(relation) => relation,
                _ => BlockRelation::Unrelated,
            }
        }
    }

    struct TestWorkingSlot {
        slot: Slot,
        fork: Vec<Slot>,
        slot_pos: usize,
    }

    impl TestWorkingSlot {
        fn new(slot: Slot, fork: &[Slot]) -> Self {
            let mut fork = fork.to_vec();
            fork.sort();
            let slot_pos = fork
                .iter()
                .position(|current| *current == slot)
                .expect("The fork didn't have the slot in it");
            TestWorkingSlot {
                slot,
                fork,
                slot_pos,
            }
        }

        fn update_slot(&mut self, slot: Slot) {
            self.slot = slot;
            self.slot_pos = self
                .fork
                .iter()
                .position(|current| *current == slot)
                .expect("The fork didn't have the slot in it");
        }
    }

    impl WorkingSlot for TestWorkingSlot {
        fn current_slot(&self) -> Slot {
            self.slot
        }

        fn is_ancestor(&self, other: Slot) -> bool {
            self.fork
                .iter()
                .position(|current| *current == other)
                .map(|other_pos| other_pos < self.slot_pos)
                .unwrap_or(false)
        }
    }

    fn new_test_loaded_program(deployment_slot: Slot, effective_slot: Slot) -> Arc<LoadedProgram> {
        new_test_loaded_program_with_usage(deployment_slot, effective_slot, AtomicU64::default())
    }
    fn new_test_loaded_program_with_usage(
        deployment_slot: Slot,
        effective_slot: Slot,
        usage_counter: AtomicU64,
    ) -> Arc<LoadedProgram> {
        Arc::new(LoadedProgram {
            program: LoadedProgramType::FailedVerification,
            account_size: 0,
            deployment_slot,
            effective_slot,
            maybe_expiration_slot: None,
            usage_counter,
        })
    }

    fn match_slot(
        table: &HashMap<Pubkey, Arc<LoadedProgram>>,
        program: &Pubkey,
        deployment_slot: Slot,
    ) -> bool {
        table
            .get(program)
            .map(|entry| entry.deployment_slot == deployment_slot)
            .unwrap_or(false)
    }

    #[test]
    fn test_fork_extract_and_prune() {
        let mut cache = LoadedPrograms::default();

        // Fork graph created for the test
        //                   0
        //                 /   \
        //                10    5
        //                |     |
        //                20    11
        //                |     | \
        //                22   15  25
        //                      |   |
        //                     16  27
        //                      |
        //                     19
        //                      |
        //                     23

        let mut fork_graph = TestForkGraphSpecific::default();
        fork_graph.insert_fork(&[0, 10, 20, 22]);
        fork_graph.insert_fork(&[0, 5, 11, 15, 16, 19, 21, 23]);
        fork_graph.insert_fork(&[0, 5, 11, 25, 27]);

        let program1 = Pubkey::new_unique();
        assert!(!cache.replenish(program1, new_test_loaded_program(0, 1)).0);
        assert!(!cache.replenish(program1, new_test_loaded_program(10, 11)).0);
        assert!(!cache.replenish(program1, new_test_loaded_program(20, 21)).0);

        // Test: inserting duplicate entry return pre existing entry from the cache
        assert!(cache.replenish(program1, new_test_loaded_program(20, 21)).0);

        let program2 = Pubkey::new_unique();
        assert!(!cache.replenish(program2, new_test_loaded_program(5, 6)).0);
        assert!(!cache.replenish(program2, new_test_loaded_program(11, 12)).0);

        let program3 = Pubkey::new_unique();
        assert!(!cache.replenish(program3, new_test_loaded_program(25, 26)).0);

        let program4 = Pubkey::new_unique();
        assert!(!cache.replenish(program4, new_test_loaded_program(0, 1)).0);
        assert!(!cache.replenish(program4, new_test_loaded_program(5, 6)).0);
        // The following is a special case, where effective slot is 3 slots in the future
        assert!(!cache.replenish(program4, new_test_loaded_program(15, 18)).0);

        // Current fork graph
        //                   0
        //                 /   \
        //                10    5
        //                |     |
        //                20    11
        //                |     | \
        //                22   15  25
        //                      |   |
        //                     16  27
        //                      |
        //                     19
        //                      |
        //                     23

        // Testing fork 0 - 10 - 12 - 22 with current slot at 22
        let working_slot = TestWorkingSlot::new(22, &[0, 10, 20, 22]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 20));
        assert!(match_slot(&found, &program4, 0));

        assert!(missing.contains(&program2));
        assert!(missing.contains(&program3));

        // Testing fork 0 - 5 - 11 - 15 - 16 with current slot at 16
        let mut working_slot = TestWorkingSlot::new(16, &[0, 5, 11, 15, 16, 18, 19, 23]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));

        // The effective slot of program4 deployed in slot 15 is 19. So it should not be usable in slot 16.
        assert!(match_slot(&found, &program4, 5));

        assert!(missing.contains(&program3));

        // Testing the same fork above, but current slot is now 18 (equal to effective slot of program4).
        working_slot.update_slot(18);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));

        // The effective slot of program4 deployed in slot 15 is 18. So it should be usable in slot 18.
        assert!(match_slot(&found, &program4, 15));

        assert!(missing.contains(&program3));

        // Testing the same fork above, but current slot is now 23 (future slot than effective slot of program4).
        working_slot.update_slot(23);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));

        // The effective slot of program4 deployed in slot 15 is 19. So it should be usable in slot 23.
        assert!(match_slot(&found, &program4, 15));

        assert!(missing.contains(&program3));

        // Testing fork 0 - 5 - 11 - 15 - 16 with current slot at 11
        let working_slot = TestWorkingSlot::new(11, &[0, 5, 11, 15, 16]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 5));
        assert!(match_slot(&found, &program4, 5));

        assert!(missing.contains(&program3));

        // The following is a special case, where there's an expiration slot
        let test_program = Arc::new(LoadedProgram {
            program: LoadedProgramType::DelayVisibility,
            account_size: 0,
            deployment_slot: 19,
            effective_slot: 19,
            maybe_expiration_slot: Some(21),
            usage_counter: AtomicU64::default(),
        });
        assert!(!cache.replenish(program4, test_program).0);

        // Testing fork 0 - 5 - 11 - 15 - 16 - 19 - 21 - 23 with current slot at 19
        let working_slot = TestWorkingSlot::new(19, &[0, 5, 11, 15, 16, 18, 19, 21, 23]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));
        // Program4 deployed at slot 19 should not be expired yet
        assert!(match_slot(&found, &program4, 19));

        assert!(missing.contains(&program3));

        // Testing fork 0 - 5 - 11 - 15 - 16 - 19 - 21 - 23 with current slot at 21
        // This would cause program4 deployed at slot 19 to be expired.
        let working_slot = TestWorkingSlot::new(21, &[0, 5, 11, 15, 16, 18, 19, 21, 23]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));

        assert!(missing.contains(&program3));
        assert!(missing.contains(&program4));

        // Remove the expired entry to let the rest of the test continue
        if let Some(programs) = cache.entries.get_mut(&program4) {
            programs.pop();
        }

        cache.prune(&fork_graph, 5);

        // Fork graph after pruning
        //                   0
        //                   |
        //                   5
        //                   |
        //                   11
        //                   | \
        //                  15  25
        //                   |   |
        //                  16  27
        //                   |
        //                  19
        //                   |
        //                  23

        // Testing fork 0 - 10 - 12 - 22 (which was pruned) with current slot at 22
        let working_slot = TestWorkingSlot::new(22, &[0, 10, 20, 22]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        // Since the fork was pruned, we should not find the entry deployed at slot 20.
        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program4, 0));

        assert!(missing.contains(&program2));
        assert!(missing.contains(&program3));

        // Testing fork 0 - 5 - 11 - 25 - 27 with current slot at 27
        let working_slot = TestWorkingSlot::new(27, &[0, 5, 11, 25, 27]);
        let (found, _missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));
        assert!(match_slot(&found, &program3, 25));
        assert!(match_slot(&found, &program4, 5));

        cache.prune(&fork_graph, 15);

        // Fork graph after pruning
        //                  0
        //                  |
        //                  5
        //                  |
        //                  11
        //                  |
        //                  15
        //                  |
        //                  16
        //                  |
        //                  19
        //                  |
        //                  23

        // Testing fork 0 - 5 - 11 - 25 - 27 (with root at 15, slot 25, 27 are pruned) with current slot at 27
        let working_slot = TestWorkingSlot::new(27, &[0, 5, 11, 25, 27]);
        let (found, missing) = cache.extract(
            &working_slot,
            vec![program1, program2, program3, program4].into_iter(),
        );

        assert!(match_slot(&found, &program1, 0));
        assert!(match_slot(&found, &program2, 11));
        assert!(match_slot(&found, &program4, 5));

        // program3 was deployed on slot 25, which has been pruned
        assert!(missing.contains(&program3));
    }
}
