use std::{collections::BTreeMap, error::Error, fmt};

use crate::{
    encounter::BattleSide,
    field_menu_domain::FieldMenuCatalog,
    game_state::GameState,
    gameplay_rng::GameplayRng,
    runtime_member::RuntimeLevelUp,
    runtime_repository::RepositoryError,
    scenario_balance::BalanceData,
    scenario_enemy::{EnemyLootEntry, MagicCoreSize},
};

use super::model::BattleState;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct BattleRewards {
    pub(super) total_experience: u32,
    pub(super) gp_gained: u32,
    pub(super) members: Vec<MemberReward>,
    pub(super) loot: Vec<LootReward>,
    pub(super) boss_flag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct MemberReward {
    pub(super) member_id: String,
    pub(super) member_name: String,
    pub(super) experience_gained: u32,
    pub(super) experience_applied: u32,
    pub(super) level_ups: Vec<RuntimeLevelUp>,
    pub(super) learned_abilities: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct LootReward {
    pub(super) item_id: String,
    pub(super) name: String,
    pub(super) quantity: u32,
    pub(super) magic_core: bool,
}

pub(super) fn calculate_rewards(state: &BattleState, rng: &mut GameplayRng) -> BattleRewards {
    let total_experience = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Enemy)
        .map(|actor| actor.experience_yield)
        .fold(0_u32, u32::saturating_add);
    let party = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party)
        .collect::<Vec<_>>();
    let living = party
        .iter()
        .filter(|actor| actor.is_alive())
        .map(|actor| actor.key)
        .collect::<Vec<_>>();
    let share = total_experience / u32::try_from(living.len()).unwrap_or(1).max(1);
    let mut bonus_pool = living.clone();
    let mut bonuses = Vec::new();
    let remainder = total_experience.saturating_sub(share.saturating_mul(living.len() as u32));
    for _ in 0..remainder {
        if bonus_pool.is_empty() {
            break;
        }
        let index = (rng.next_u64() % bonus_pool.len() as u64) as usize;
        bonuses.push(bonus_pool.remove(index));
    }
    let members = party
        .into_iter()
        .map(|actor| {
            let experience_gained = if actor.is_alive() {
                share + u32::from(bonuses.contains(&actor.key))
            } else {
                0
            };
            MemberReward {
                member_id: actor.id.clone(),
                member_name: actor.name.clone(),
                experience_gained,
                experience_applied: 0,
                level_ups: Vec::new(),
                learned_abilities: Vec::new(),
            }
        })
        .collect();
    BattleRewards {
        total_experience,
        gp_gained: 0,
        members,
        loot: calculate_loot(state, rng),
        boss_flag: None,
    }
}

pub(super) fn apply_rewards(
    state: &mut BattleState,
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    balance: &BalanceData,
    configured_boss_flag: Option<&str>,
) -> Result<BattleRewards, RewardError> {
    if state.rewards.is_some() {
        return Err(RewardError::AlreadyApplied);
    }
    let mut staged = game.clone();
    let mut rewards = calculate_rewards(state, staged.rng_mut());
    let flags = staged.flags().clone();
    for result in &mut rewards.members {
        let battle_actor = state
            .combatants
            .iter()
            .find(|actor| actor.key.side == BattleSide::Party && actor.id == result.member_id)
            .ok_or_else(|| RewardError::MissingPartyMember(result.member_id.clone()))?;
        let member = staged
            .party_mut()
            .member_mut(&result.member_id)
            .ok_or_else(|| RewardError::MissingPartyMember(result.member_id.clone()))?;
        member.sync_battle_pools(battle_actor.health, battle_actor.mana);
        let class = catalog
            .class(member.class_id())
            .ok_or_else(|| RewardError::MissingClass(member.class_id().to_owned()))?;
        let old_level = member.level();
        let progression = member.apply_experience_progression(
            result.experience_gained,
            class,
            &balance.progression,
        );
        result.experience_applied = progression.added;
        result.level_ups = progression.level_ups;
        let new_level = member.level();
        result.learned_abilities = catalog
            .unlocked_abilities(member.class_id(), new_level, &flags)
            .into_iter()
            .filter(|ability| ability.unlock_level.get() > old_level)
            .map(|ability| ability.name.clone())
            .collect();
    }

    let batch = staged.repository_mut().start_loot_batch();
    for loot in &mut rewards.loot {
        let tags = loot.magic_core.then(|| "magic_core".to_owned());
        let outcome = staged.repository_mut().add_loot_item_in_batch(
            loot.item_id.clone(),
            loot.quantity,
            batch,
            tags,
        )?;
        loot.quantity = outcome.added();
    }
    rewards.loot.retain(|loot| loot.quantity > 0);
    if rewards.gp_gained > 0 {
        let outcome = staged.repository_mut().add_gp(rewards.gp_gained)?;
        rewards.gp_gained = outcome.added();
    }
    let boss_defeated = state
        .combatants
        .iter()
        .any(|actor| actor.key.side == BattleSide::Enemy && actor.boss && !actor.is_alive());
    if boss_defeated && let Some(flag) = configured_boss_flag.filter(|flag| !flag.is_empty()) {
        staged.flags_mut().set(flag);
        rewards.boss_flag = Some(flag.to_owned());
    }
    for actor in state
        .combatants
        .iter_mut()
        .filter(|actor| actor.key.side == BattleSide::Party)
    {
        let member = staged
            .party()
            .member(&actor.id)
            .expect("reward party members validated before commit");
        actor.health = member.health();
        actor.max_health = member.max_health();
        actor.mana = member.mana();
        actor.max_mana = member.max_mana();
    }
    *game = staged;
    state.rewards = Some(rewards.clone());
    Ok(rewards)
}

impl BattleRewards {
    pub(super) fn summary_lines(&self) -> Vec<String> {
        let member_exp = self
            .members
            .iter()
            .map(|member| {
                let capped = if member.experience_applied != member.experience_gained {
                    format!(" ({} earned)", member.experience_gained)
                } else {
                    String::new()
                };
                format!(
                    "{} +{} EXP{capped}",
                    member.member_name, member.experience_applied
                )
            })
            .collect::<Vec<_>>()
            .join("  ");
        let levels = self
            .members
            .iter()
            .filter_map(|member| {
                let first = member.level_ups.first()?;
                let last = member.level_ups.last()?;
                Some(format!(
                    "{} Lv {}>{}",
                    member.member_name, first.old_level, last.new_level,
                ))
            })
            .collect::<Vec<_>>()
            .join("  ");
        let learned = self
            .members
            .iter()
            .flat_map(|member| {
                member
                    .learned_abilities
                    .iter()
                    .map(|ability| format!("{} learned {ability}", member.member_name))
            })
            .collect::<Vec<_>>()
            .join("  ");
        let loot = self
            .loot
            .iter()
            .map(|loot| format!("{} x{}", loot.name, loot.quantity))
            .collect::<Vec<_>>()
            .join("  ");
        let boss = self
            .boss_flag
            .as_ref()
            .map(|flag| format!("Boss cleared ({flag})"))
            .unwrap_or_default();
        vec![
            format!("EXP {}  GP {}", self.total_experience, self.gp_gained),
            if member_exp.is_empty() {
                "No EXP".to_owned()
            } else {
                member_exp
            },
            if loot.is_empty() {
                "No loot".to_owned()
            } else {
                loot
            },
            [levels, learned, boss]
                .into_iter()
                .filter(|line| !line.is_empty())
                .collect::<Vec<_>>()
                .join("  "),
        ]
    }

    pub(super) fn detail_message(&self) -> String {
        let details = self
            .members
            .iter()
            .filter_map(|member| {
                let first = member.level_ups.first()?;
                let last = member.level_ups.last()?;
                let sum = |pick: fn(&RuntimeLevelUp) -> u32| {
                    member.level_ups.iter().map(pick).sum::<u32>()
                };
                Some(format!(
                    "{} Lv {}>{}\nHP +{}={}  MP +{}={}\nSTR +{}={}  DEX +{}={}  CON +{}={}  INT +{}={}",
                    member.member_name,
                    first.old_level,
                    last.new_level,
                    sum(|level| level.health),
                    last.max_health,
                    sum(|level| level.mana),
                    last.max_mana,
                    sum(|level| level.strength),
                    last.total_strength,
                    sum(|level| level.dexterity),
                    last.total_dexterity,
                    sum(|level| level.constitution),
                    last.total_constitution,
                    sum(|level| level.intelligence),
                    last.total_intelligence,
                ))
            })
            .collect::<Vec<_>>()
            .join("\n\n");
        if details.is_empty() {
            "Rewards applied. Press Enter to continue.".to_owned()
        } else {
            format!("{details}\n\nPress Enter to continue.")
        }
    }
}

fn calculate_loot(state: &BattleState, rng: &mut GameplayRng) -> Vec<LootReward> {
    let mut totals = BTreeMap::<String, (u32, bool)>::new();
    for enemy in state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Enemy)
    {
        let Some(drops) = &enemy.drops else { continue };
        for core in &drops.mc {
            let id = magic_core_id(core.size).to_owned();
            let entry = totals.entry(id).or_insert((0, true));
            entry.0 = entry.0.saturating_add(core.qty.get());
        }
        for pool in &drops.loot {
            if let Some(drop) = pick_loot(&pool.pool, rng) {
                let entry = totals.entry(drop.item.clone()).or_insert((0, false));
                entry.0 = entry.0.saturating_add(1);
            }
        }
    }
    totals
        .into_iter()
        .map(|(item_id, (quantity, magic_core))| LootReward {
            name: if magic_core {
                magic_core_name(&item_id).to_owned()
            } else {
                display_item_name(&item_id)
            },
            item_id,
            quantity,
            magic_core,
        })
        .collect()
}

fn pick_loot<'a>(pool: &'a [EnemyLootEntry], rng: &mut GameplayRng) -> Option<&'a EnemyLootEntry> {
    let total = pool
        .iter()
        .map(|entry| u64::from(entry.weight.get()))
        .sum::<u64>();
    if total == 0 {
        return None;
    }
    let mut roll = rng.next_u64() % total;
    for entry in pool {
        let weight = u64::from(entry.weight.get());
        if roll < weight {
            return Some(entry);
        }
        roll -= weight;
    }
    None
}

fn magic_core_id(size: MagicCoreSize) -> &'static str {
    match size {
        MagicCoreSize::ExtraSmall => "mc_xs",
        MagicCoreSize::Small => "mc_s",
        MagicCoreSize::Medium => "mc_m",
        MagicCoreSize::Large => "mc_l",
        MagicCoreSize::ExtraLarge => "mc_xl",
    }
}

fn magic_core_name(id: &str) -> &'static str {
    match id {
        "mc_xs" => "Magic Core (XS)",
        "mc_s" => "Magic Core (S)",
        "mc_m" => "Magic Core (M)",
        "mc_l" => "Magic Core (L)",
        "mc_xl" => "Magic Core (XL)",
        _ => "Magic Core",
    }
}

fn display_item_name(id: &str) -> String {
    id.split('_')
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[derive(Debug)]
pub(super) enum RewardError {
    AlreadyApplied,
    MissingPartyMember(String),
    MissingClass(String),
    Repository(RepositoryError),
}

impl From<RepositoryError> for RewardError {
    fn from(value: RepositoryError) -> Self {
        Self::Repository(value)
    }
}

impl fmt::Display for RewardError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyApplied => formatter.write_str("battle rewards were already applied"),
            Self::MissingPartyMember(id) => {
                write!(formatter, "reward party member `{id}` is missing")
            }
            Self::MissingClass(id) => write!(formatter, "reward class `{id}` is missing"),
            Self::Repository(error) => {
                write!(formatter, "reward repository update failed: {error}")
            }
        }
    }
}

impl Error for RewardError {}
