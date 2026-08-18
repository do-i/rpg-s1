//! Deterministic minimum-complete battle loop (Milestone 9).

use bevy::{asset::LoadState, prelude::*};

use crate::{
    action_input::{ActionState, AppAction},
    app_state::{AppState, AppStateTransitionRequest},
    encounter::{BattleEntry, BattleParticipant, BattleSide, restore_pre_battle_context},
    game_state::GameState,
    gameplay_rng::GameplayRng,
    scenario_balance::BalanceData,
    scenario_party::PartyRow,
    scenario_path::ScenarioRelativePath,
    scenario_root::ScenarioRoot,
    tsx_atlas_asset::TsxAtlasAsset,
    world_encounter::WorldEncounterRestore,
};

const BATTLE_IDLE_TILE: u32 = 8 * 9;
const COMMANDS: [BattleCommand; 4] = [
    BattleCommand::Attack,
    BattleCommand::Spell,
    BattleCommand::Item,
    BattleCommand::Run,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BattlePhase {
    Start,
    Command,
    Target,
    Resolve,
    Advance,
    Victory,
    Defeat,
    Flee,
}

impl BattlePhase {
    pub(crate) const fn allows(self, next: Self) -> bool {
        match self {
            Self::Start => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Command => matches!(next, Self::Target | Self::Victory | Self::Flee),
            Self::Target => matches!(next, Self::Command | Self::Resolve | Self::Victory),
            Self::Resolve => matches!(next, Self::Advance | Self::Victory | Self::Defeat),
            Self::Advance => matches!(next, Self::Command | Self::Resolve | Self::Defeat),
            Self::Victory | Self::Defeat => false,
            Self::Flee => matches!(next, Self::Advance),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BattleCommand {
    Attack,
    Spell,
    Item,
    Run,
}

impl BattleCommand {
    const fn label(self) -> &'static str {
        match self {
            Self::Attack => "Attack",
            Self::Spell => "Spell",
            Self::Item => "Item",
            Self::Run => "Run",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct CombatantKey {
    side: BattleSide,
    index: usize,
}

impl CombatantKey {
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "M9 ally-target contract is exercised before M10 effects"
        )
    )]
    const fn party(index: usize) -> Self {
        Self {
            side: BattleSide::Party,
            index,
        }
    }

    const fn enemy(index: usize) -> Self {
        Self {
            side: BattleSide::Enemy,
            index,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct BattleCombatant {
    key: CombatantKey,
    id: String,
    name: String,
    class_id: String,
    health: u32,
    max_health: u32,
    mana: u32,
    max_mana: u32,
    attack: i64,
    defense: i64,
    dexterity: i64,
    row: PartyRow,
    boss: bool,
}

impl BattleCombatant {
    fn from_participant(participant: &BattleParticipant, index: usize) -> Self {
        Self {
            key: CombatantKey {
                side: participant.side,
                index,
            },
            id: participant.id.clone(),
            name: participant.name.clone(),
            class_id: participant.class_id.clone(),
            health: participant.health,
            max_health: participant.max_health,
            mana: participant.mana,
            max_mana: participant.max_mana,
            attack: participant.attack,
            defense: participant.defense,
            dexterity: participant.dexterity,
            row: participant.row,
            boss: participant.boss,
        }
    }

    const fn is_alive(&self) -> bool {
        self.health > 0
    }

    fn apply_damage(&mut self, amount: u32) -> u32 {
        let actual = amount.min(self.health);
        self.health -= actual;
        actual
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetGroup {
    Enemy,
    #[cfg_attr(
        not(test),
        expect(
            dead_code,
            reason = "M9 verifies ally eligibility before M10 effects consume it"
        )
    )]
    Ally,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TargetSelector {
    group: TargetGroup,
    eligible: Vec<CombatantKey>,
    selected: usize,
}

impl TargetSelector {
    pub(crate) fn new(
        group: TargetGroup,
        combatants: &[BattleCombatant],
        ko_eligible: bool,
    ) -> Option<Self> {
        let side = match group {
            TargetGroup::Enemy => BattleSide::Enemy,
            TargetGroup::Ally => BattleSide::Party,
        };
        let eligible = combatants
            .iter()
            .filter(|actor| actor.key.side == side)
            .filter(|actor| actor.is_alive() != ko_eligible)
            .map(|actor| actor.key)
            .collect::<Vec<_>>();
        (!eligible.is_empty()).then_some(Self {
            group,
            eligible,
            selected: 0,
        })
    }

    fn selected(&self) -> CombatantKey {
        self.eligible[self.selected]
    }

    fn navigate(&mut self, movement: isize) {
        self.selected = wrap_index(self.selected, movement, self.eligible.len());
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FleeOutcome {
    Success,
    Failed,
    Blocked,
}

#[derive(Debug, Resource)]
pub(crate) struct BattleState {
    phase: BattlePhase,
    combatants: Vec<BattleCombatant>,
    turn_order: Vec<CombatantKey>,
    active_turn: usize,
    command_index: usize,
    target: Option<TargetSelector>,
    message: String,
    transcript: Vec<String>,
    flee_outcome: Option<FleeOutcome>,
}

impl BattleState {
    fn from_entry(entry: &BattleEntry) -> Self {
        let mut party_index = 0;
        let mut enemy_index = 0;
        let combatants = entry
            .participants
            .iter()
            .map(|participant| {
                let index = match participant.side {
                    BattleSide::Party => {
                        let current = party_index;
                        party_index += 1;
                        current
                    }
                    BattleSide::Enemy => {
                        let current = enemy_index;
                        enemy_index += 1;
                        current
                    }
                };
                BattleCombatant::from_participant(participant, index)
            })
            .collect::<Vec<_>>();
        Self {
            phase: BattlePhase::Start,
            turn_order: calculate_turn_order(&combatants),
            combatants,
            active_turn: 0,
            command_index: 0,
            target: None,
            message: "Battle start!".to_owned(),
            transcript: vec![format!("START {}", entry.encounter_id)],
            flee_outcome: None,
        }
    }

    fn actor(&self, key: CombatantKey) -> Option<&BattleCombatant> {
        self.combatants.iter().find(|actor| actor.key == key)
    }

    fn actor_mut(&mut self, key: CombatantKey) -> Option<&mut BattleCombatant> {
        self.combatants.iter_mut().find(|actor| actor.key == key)
    }

    fn active_key(&self) -> Option<CombatantKey> {
        self.turn_order.get(self.active_turn).copied()
    }

    fn active(&self) -> Option<&BattleCombatant> {
        self.active_key().and_then(|key| self.actor(key))
    }

    fn all_defeated(&self, side: BattleSide) -> bool {
        self.combatants
            .iter()
            .filter(|actor| actor.key.side == side)
            .all(|actor| !actor.is_alive())
    }

    fn command_available(&self, command: BattleCommand) -> bool {
        let Some(active) = self.active() else {
            return false;
        };
        match command {
            BattleCommand::Attack => active.is_alive(),
            // Spell and item effects enter in M10. They remain visible but explicitly disabled.
            BattleCommand::Spell | BattleCommand::Item => false,
            BattleCommand::Run => active.is_alive(),
        }
    }

    fn begin_active_turn(&mut self, rng: &mut GameplayRng) {
        self.skip_knocked_out();
        let Some(active) = self.active().cloned() else {
            self.phase = BattlePhase::Defeat;
            return;
        };
        self.transcript.push(format!("TURN {}", active.id));
        if active.key.side == BattleSide::Party {
            self.phase = BattlePhase::Command;
            self.command_index = 0;
            self.message = format!("{}: choose an action.", active.name);
        } else {
            self.resolve_enemy_action(rng);
        }
    }

    fn resolve_enemy_action(&mut self, rng: &mut GameplayRng) {
        let Some(attacker_key) = self.active_key() else {
            return;
        };
        let living = self
            .combatants
            .iter()
            .filter(|actor| actor.key.side == BattleSide::Party && actor.is_alive())
            .map(|actor| actor.key)
            .collect::<Vec<_>>();
        if living.is_empty() {
            self.phase = BattlePhase::Defeat;
            return;
        }
        let target = living[(rng.next_u64() % living.len() as u64) as usize];
        self.resolve_physical(attacker_key, target, rng);
    }

    fn resolve_physical(
        &mut self,
        attacker_key: CombatantKey,
        target_key: CombatantKey,
        rng: &mut GameplayRng,
    ) {
        let Some(attacker) = self.actor(attacker_key).cloned() else {
            return;
        };
        let Some(defender) = self.actor(target_key).cloned() else {
            return;
        };
        let chance = physical_hit_chance(attacker.dexterity, defender.dexterity);
        if !roll_succeeds(rng, chance) {
            self.message = format!("{} attacks {}, but misses.", attacker.name, defender.name);
            self.transcript
                .push(format!("MISS {} -> {}", attacker.id, defender.id));
            self.phase = BattlePhase::Resolve;
            return;
        }
        let raw = physical_damage(&attacker, &defender);
        let actual = self
            .actor_mut(target_key)
            .map(|target| target.apply_damage(raw))
            .unwrap_or(0);
        let knocked_out = self
            .actor(target_key)
            .is_some_and(|target| !target.is_alive());
        self.message = format!(
            "{} attacks {} for {} damage.{}",
            attacker.name,
            defender.name,
            actual,
            if knocked_out { " KO!" } else { "" }
        );
        self.transcript.push(format!(
            "HIT {} -> {} {}{}",
            attacker.id,
            defender.id,
            actual,
            if knocked_out { " KO" } else { "" }
        ));
        self.phase = BattlePhase::Resolve;
    }

    fn assess_result(&mut self) {
        if self.all_defeated(BattleSide::Enemy) {
            self.phase = BattlePhase::Victory;
            self.message = "Victory! Press Enter to return to the world.".to_owned();
            self.transcript.push("VICTORY".to_owned());
        } else if self.all_defeated(BattleSide::Party) {
            self.phase = BattlePhase::Defeat;
            self.message = "The party was defeated.".to_owned();
            self.transcript.push("DEFEAT".to_owned());
        } else {
            self.phase = BattlePhase::Advance;
        }
    }

    fn advance(&mut self, rng: &mut GameplayRng) {
        if !self.turn_order.is_empty() {
            self.active_turn = (self.active_turn + 1) % self.turn_order.len();
        }
        self.begin_active_turn(rng);
    }

    fn skip_knocked_out(&mut self) {
        for _ in 0..self.turn_order.len() {
            if self.active().is_some_and(BattleCombatant::is_alive) {
                break;
            }
            self.active_turn = (self.active_turn + 1) % self.turn_order.len();
        }
    }
}

pub(crate) fn calculate_turn_order(combatants: &[BattleCombatant]) -> Vec<CombatantKey> {
    let mut actors = combatants
        .iter()
        .filter(|actor| actor.is_alive())
        .collect::<Vec<_>>();
    actors.sort_by(|left, right| {
        right
            .dexterity
            .cmp(&left.dexterity)
            .then_with(|| side_priority(left.key.side).cmp(&side_priority(right.key.side)))
            .then_with(|| left.key.index.cmp(&right.key.index))
    });
    actors.into_iter().map(|actor| actor.key).collect()
}

const fn side_priority(side: BattleSide) -> u8 {
    match side {
        BattleSide::Party => 0,
        BattleSide::Enemy => 1,
    }
}

pub(crate) fn physical_hit_chance(attacker_dex: i64, defender_dex: i64) -> f64 {
    (0.70 + (attacker_dex - defender_dex) as f64 * 0.02).clamp(0.05, 0.95)
}

pub(crate) fn physical_damage(attacker: &BattleCombatant, defender: &BattleCombatant) -> u32 {
    let mut damage = (attacker.attack - defender.defense).max(1) as u32;
    if attacker.row == PartyRow::Back {
        damage = (damage / 2).max(1);
    }
    if defender.key.side == BattleSide::Party && defender.row == PartyRow::Back {
        damage = (damage / 2).max(1);
    }
    damage.min(defender.health)
}

fn roll_succeeds(rng: &mut GameplayRng, chance: f64) -> bool {
    let roll = (rng.next_u64() >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64));
    roll < chance
}

fn roll_flee(boss: bool, chance: f64, rng: &mut GameplayRng) -> FleeOutcome {
    if boss {
        FleeOutcome::Blocked
    } else if chance >= 1.0 || roll_succeeds(rng, chance) {
        FleeOutcome::Success
    } else {
        FleeOutcome::Failed
    }
}

const fn phase_after_flee_confirmation(outcome: FleeOutcome) -> Option<BattlePhase> {
    match outcome {
        FleeOutcome::Success => None,
        FleeOutcome::Failed | FleeOutcome::Blocked => Some(BattlePhase::Advance),
    }
}

fn wrap_index(index: usize, movement: isize, length: usize) -> usize {
    (index as isize + movement).rem_euclid(length as isize) as usize
}

pub(crate) fn flee_chance(state: &BattleState, balance: &BalanceData) -> f64 {
    let rogue_dexterity = state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party && actor.class_id == "rogue")
        .map(|actor| actor.dexterity.max(0) as f64)
        .sum::<f64>();
    (balance.battle.flee_base_chance.get()
        + rogue_dexterity * balance.battle.flee_rogue_dex_bonus.get())
    .min(1.0)
}

pub(crate) struct BattlePlugin;

impl Plugin for BattlePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(AppState::Battle), enter_battle)
            .add_systems(
                Update,
                (drive_battle_assets, handle_battle_input, sync_battle_ui)
                    .chain()
                    .run_if(in_state(AppState::Battle)),
            )
            .add_systems(OnExit(AppState::Battle), cleanup_battle);
    }
}

#[derive(Component)]
struct BattleUi;

#[derive(Component)]
struct BattleEnemyImage(usize);

#[derive(Component)]
struct BattleEnemyLabel(usize);

#[derive(Component)]
struct BattlePartyText;

#[derive(Component)]
struct BattleCommandText;

#[derive(Component)]
struct BattleMessageText;

#[derive(Debug, Resource)]
struct BattleAssetState {
    atlases: Vec<Option<Handle<TsxAtlasAsset>>>,
}

fn enter_battle(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    root: Res<ScenarioRoot>,
    entry: Option<Res<BattleEntry>>,
) {
    let Some(entry) = entry else {
        return;
    };
    let state = BattleState::from_entry(&entry);
    let atlases = entry
        .participants
        .iter()
        .filter(|participant| participant.side == BattleSide::Enemy)
        .map(|participant| {
            ScenarioRelativePath::try_from(
                format!("assets/sprites/enemies/{}.tsx", participant.sprite_id).as_str(),
            )
            .ok()
            .map(|path| asset_server.load(root.resolve(&path)))
        })
        .collect::<Vec<_>>();
    let background = ScenarioRelativePath::try_from(entry.background_asset.as_str())
        .ok()
        .map(|path| asset_server.load(root.resolve(&path)))
        .unwrap_or_default();
    let font = asset_server.load(
        root.resolve(
            &ScenarioRelativePath::try_from("assets/fonts/Philosopher-Regular.ttf")
                .expect("battle font path"),
        ),
    );
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                width: percent(100),
                height: percent(100),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::all(px(24)),
                ..default()
            },
            ImageNode::new(background).with_mode(NodeImageMode::Stretch),
            BackgroundColor(Color::srgb(0.03, 0.03, 0.08)),
            GlobalZIndex(200),
            Pickable::IGNORE,
            BattleUi,
        ))
        .with_children(|root_node| {
            root_node
                .spawn(Node {
                    width: percent(100),
                    height: px(390),
                    justify_content: JustifyContent::SpaceEvenly,
                    align_items: AlignItems::Center,
                    ..default()
                })
                .with_children(|row| {
                    for (index, participant) in entry
                        .participants
                        .iter()
                        .filter(|participant| participant.side == BattleSide::Enemy)
                        .enumerate()
                    {
                        row.spawn(Node {
                            width: px(180),
                            height: px(240),
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            row_gap: px(8),
                            ..default()
                        })
                        .with_children(|card| {
                            card.spawn((
                                ImageNode::solid_color(Color::srgba(0.7, 0.1, 0.1, 0.7)),
                                Node {
                                    width: px(128),
                                    height: px(128),
                                    ..default()
                                },
                                BattleEnemyImage(index),
                            ));
                            card.spawn((
                                Text::new(participant.name.clone()),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Px(22.0),
                                    ..default()
                                },
                                TextColor(Color::WHITE),
                                BattleEnemyLabel(index),
                            ));
                        });
                    }
                });
            root_node
                .spawn((
                    Node {
                        width: percent(100),
                        height: px(285),
                        flex_direction: FlexDirection::Row,
                        padding: UiRect::all(px(18)),
                        column_gap: px(28),
                        border_radius: BorderRadius::all(px(12)),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.015, 0.02, 0.09, 0.92)),
                ))
                .with_children(|panel| {
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(235, 225, 190)),
                        Node {
                            width: percent(48),
                            ..default()
                        },
                        BattlePartyText,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Px(20.0),
                            ..default()
                        },
                        TextColor(Color::WHITE),
                        Node {
                            width: percent(20),
                            ..default()
                        },
                        BattleCommandText,
                    ));
                    panel.spawn((
                        Text::new(""),
                        TextFont {
                            font: font.into(),
                            font_size: FontSize::Px(19.0),
                            ..default()
                        },
                        TextColor(Color::srgb_u8(117, 220, 214)),
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                        BattleMessageText,
                    ));
                });
        });
    commands.insert_resource(BattleAssetState { atlases });
    commands.insert_resource(state);
}

fn drive_battle_assets(
    asset_server: Res<AssetServer>,
    atlases: Res<Assets<TsxAtlasAsset>>,
    assets: Option<Res<BattleAssetState>>,
    mut images: Query<(&BattleEnemyImage, &mut ImageNode)>,
) {
    let Some(assets) = assets else { return };
    for (marker, mut image) in &mut images {
        let Some(Some(handle)) = assets.atlases.get(marker.0) else {
            continue;
        };
        if matches!(asset_server.load_state(handle.id()), LoadState::Failed(_)) {
            continue;
        }
        let Some(atlas) = atlases.get(handle) else {
            continue;
        };
        if let Ok(sprite) = atlas.sprite_for_tile(BATTLE_IDLE_TILE) {
            image.image = sprite.image;
            image.texture_atlas = sprite.texture_atlas;
            image.color = Color::WHITE;
        }
    }
}

fn handle_battle_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    balances: Res<Assets<BalanceData>>,
    mut game: Option<ResMut<GameState>>,
    entry: Option<Res<BattleEntry>>,
    state: Option<ResMut<BattleState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
) {
    let (Some(entry), Some(mut state), Some(mut game)) = (entry, state, game.take()) else {
        return;
    };
    match state.phase {
        BattlePhase::Start => state.begin_active_turn(game.rng_mut()),
        BattlePhase::Advance => state.advance(game.rng_mut()),
        BattlePhase::Command => {
            if actions.just_pressed(AppAction::Back) {
                attempt_flee(&mut state, &balances, &mut game);
                return;
            }
            if let Some(movement) = actions.menu_navigation() {
                state.command_index = wrap_index(state.command_index, movement, COMMANDS.len());
            }
            if actions.just_pressed(AppAction::Confirm) {
                let command = COMMANDS[state.command_index];
                if !state.command_available(command) {
                    state.message = match command {
                        BattleCommand::Spell | BattleCommand::Item => {
                            format!("{} effects unlock in Milestone 10.", command.label())
                        }
                        BattleCommand::Run => "Can't escape from a boss!".to_owned(),
                        BattleCommand::Attack => "That action is unavailable.".to_owned(),
                    };
                    return;
                }
                match command {
                    BattleCommand::Attack => {
                        state.target =
                            TargetSelector::new(TargetGroup::Enemy, &state.combatants, false);
                        if state.target.is_some() {
                            debug_assert!(state.phase.allows(BattlePhase::Target));
                            state.phase = BattlePhase::Target;
                            state.message = "Choose a living enemy. ESC cancels.".to_owned();
                        } else {
                            state.assess_result();
                        }
                    }
                    BattleCommand::Run => {
                        attempt_flee(&mut state, &balances, &mut game);
                    }
                    BattleCommand::Spell | BattleCommand::Item => {}
                }
            }
        }
        BattlePhase::Target => {
            if actions.just_pressed(AppAction::Back) {
                state.target = None;
                state.phase = BattlePhase::Command;
                state.message = "Action cancelled.".to_owned();
            } else {
                if let Some(movement) = actions.menu_navigation()
                    && let Some(target) = state.target.as_mut()
                {
                    target.navigate(movement);
                }
                if actions.just_pressed(AppAction::Confirm) {
                    let attacker = state.active_key();
                    let target = state.target.as_ref().map(TargetSelector::selected);
                    state.target = None;
                    if let (Some(attacker), Some(target)) = (attacker, target) {
                        state.resolve_physical(attacker, target, game.rng_mut());
                    }
                }
            }
        }
        BattlePhase::Resolve if actions.just_pressed(AppAction::Confirm) => state.assess_result(),
        BattlePhase::Victory if actions.just_pressed(AppAction::Confirm) => {
            apply_victory(&mut commands, &mut game, &entry, &state);
            transitions.write(AppStateTransitionRequest::new(AppState::World));
        }
        BattlePhase::Defeat => {
            transitions.write(AppStateTransitionRequest::new(AppState::GameOver));
        }
        BattlePhase::Flee if actions.just_pressed(AppAction::Confirm) => {
            let outcome = state.flee_outcome.unwrap_or(FleeOutcome::Failed);
            if phase_after_flee_confirmation(outcome).is_none() {
                restore_world(&mut commands, &mut game, &entry);
                transitions.write(AppStateTransitionRequest::new(AppState::World));
            } else {
                state.phase = phase_after_flee_confirmation(outcome)
                    .expect("failed and blocked flee consume the active turn");
            }
        }
        _ => {}
    }
}

fn attempt_flee(state: &mut BattleState, balances: &Assets<BalanceData>, game: &mut GameState) {
    let Some(balance) = balances.iter().next().map(|(_, value)| value) else {
        state.message = "Battle balance is still loading.".to_owned();
        return;
    };
    let boss = state
        .combatants
        .iter()
        .any(|actor| actor.key.side == BattleSide::Enemy && actor.boss);
    let chance = flee_chance(state, balance);
    let outcome = roll_flee(boss, chance, game.rng_mut());
    state.flee_outcome = Some(outcome);
    state.phase = BattlePhase::Flee;
    state.message = match outcome {
        FleeOutcome::Success => "Got away safely! Press Enter.".to_owned(),
        FleeOutcome::Failed => "Couldn't escape! Press Enter.".to_owned(),
        FleeOutcome::Blocked => "Can't escape from a boss! Press Enter.".to_owned(),
    };
    state.transcript.push(format!("FLEE {outcome:?}"));
}

fn apply_victory(
    commands: &mut Commands,
    game: &mut GameState,
    entry: &BattleEntry,
    state: &BattleState,
) {
    for actor in state
        .combatants
        .iter()
        .filter(|actor| actor.key.side == BattleSide::Party)
    {
        if let Some(member) = game.party_mut().member_mut(&actor.id) {
            member.apply_damage(member.health().saturating_sub(actor.health));
            member.spend_mana(member.mana().saturating_sub(actor.mana));
        }
    }
    if let Some(flag) = entry.boss_completion_flag.as_deref()
        && !flag.is_empty()
    {
        game.flags_mut().set(flag);
    }
    restore_world(commands, game, entry);
}

fn restore_world(commands: &mut Commands, game: &mut GameState, entry: &BattleEntry) {
    if restore_pre_battle_context(game, &entry.return_context).is_ok() {
        commands.insert_resource(WorldEncounterRestore {
            map_id: entry.return_context.map_id.clone(),
            enemies: entry.return_context.world_enemies.clone(),
        });
    }
}

#[expect(
    clippy::type_complexity,
    reason = "disjoint UI marker filters prevent mutable Text query overlap"
)]
fn sync_battle_ui(
    state: Option<Res<BattleState>>,
    mut party_text: Query<
        &mut Text,
        (
            With<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut command_text: Query<
        &mut Text,
        (
            With<BattleCommandText>,
            Without<BattlePartyText>,
            Without<BattleMessageText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut message_text: Query<
        &mut Text,
        (
            With<BattleMessageText>,
            Without<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleEnemyLabel>,
        ),
    >,
    mut enemy_images: Query<(&BattleEnemyImage, &mut ImageNode)>,
    mut enemy_labels: Query<
        (&BattleEnemyLabel, &mut Text, &mut TextColor),
        (
            Without<BattlePartyText>,
            Without<BattleCommandText>,
            Without<BattleMessageText>,
        ),
    >,
) {
    let Some(state) = state else { return };
    if !state.is_changed() {
        return;
    }
    let active = state.active_key();
    if let Ok(mut text) = party_text.single_mut() {
        text.0 = state
            .combatants
            .iter()
            .filter(|actor| actor.key.side == BattleSide::Party)
            .map(|actor| {
                format!(
                    "{}{}\n  HP {}/{}  MP {}/{}  {}{}",
                    if active == Some(actor.key) {
                        "> "
                    } else {
                        "  "
                    },
                    actor.name,
                    actor.health,
                    actor.max_health,
                    actor.mana,
                    actor.max_mana,
                    match actor.row {
                        PartyRow::Front => "Front",
                        PartyRow::Back => "Back",
                    },
                    if actor.is_alive() { "" } else { "  KO" },
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Ok(mut text) = command_text.single_mut() {
        text.0 = COMMANDS
            .iter()
            .enumerate()
            .map(|(index, command)| {
                let selected = state.phase == BattlePhase::Command && index == state.command_index;
                let available = state.command_available(*command);
                format!(
                    "{}{}{}",
                    if selected { "> " } else { "  " },
                    command.label(),
                    if available { "" } else { " [--]" }
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");
    }
    if let Ok(mut text) = message_text.single_mut() {
        let target = state
            .target
            .as_ref()
            .and_then(|selector| state.actor(selector.selected()))
            .map(|actor| format!("\n\nTarget: {}", actor.name))
            .unwrap_or_default();
        text.0 = format!("{:?}\n\n{}{}", state.phase, state.message, target);
    }
    let selected_target = state.target.as_ref().map(TargetSelector::selected);
    for (marker, mut image) in &mut enemy_images {
        let key = CombatantKey::enemy(marker.0);
        let Some(enemy) = state.actor(key) else {
            continue;
        };
        image.color = if !enemy.is_alive() {
            Color::srgba(0.3, 0.3, 0.3, 0.45)
        } else if selected_target == Some(key) {
            Color::srgb(1.0, 0.85, 0.35)
        } else {
            Color::WHITE
        };
    }
    for (marker, mut text, mut color) in &mut enemy_labels {
        let key = CombatantKey::enemy(marker.0);
        let Some(enemy) = state.actor(key) else {
            continue;
        };
        text.0 = format!(
            "{}  HP {}/{}{}",
            enemy.name,
            enemy.health,
            enemy.max_health,
            if enemy.is_alive() { "" } else { "  KO" }
        );
        color.0 = if selected_target == Some(key) {
            Color::srgb_u8(255, 220, 90)
        } else {
            Color::WHITE
        };
    }
}

fn cleanup_battle(mut commands: Commands, entities: Query<Entity, With<BattleUi>>) {
    for entity in &entities {
        commands.entity(entity).despawn();
    }
    commands.remove_resource::<BattleState>();
    commands.remove_resource::<BattleAssetState>();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(side: BattleSide, index: usize, dex: i64, health: u32) -> BattleCombatant {
        BattleCombatant {
            key: CombatantKey { side, index },
            id: format!("{side:?}-{index}"),
            name: format!("{side:?} {index}"),
            class_id: String::new(),
            health,
            max_health: health.max(1),
            mana: 0,
            max_mana: 0,
            attack: 10,
            defense: 3,
            dexterity: dex,
            row: PartyRow::Front,
            boss: false,
        }
    }

    fn state_with(combatants: Vec<BattleCombatant>) -> BattleState {
        BattleState {
            phase: BattlePhase::Command,
            turn_order: calculate_turn_order(&combatants),
            combatants,
            active_turn: 0,
            command_index: 0,
            target: None,
            message: String::new(),
            transcript: Vec::new(),
            flee_outcome: None,
        }
    }

    #[test]
    fn phase_graph_names_and_bounds_every_minimum_loop_transition() {
        assert!(BattlePhase::Start.allows(BattlePhase::Command));
        assert!(BattlePhase::Command.allows(BattlePhase::Target));
        assert!(BattlePhase::Target.allows(BattlePhase::Resolve));
        assert!(BattlePhase::Resolve.allows(BattlePhase::Advance));
        assert!(BattlePhase::Advance.allows(BattlePhase::Command));
        assert!(BattlePhase::Resolve.allows(BattlePhase::Victory));
        assert!(BattlePhase::Resolve.allows(BattlePhase::Defeat));
        assert!(BattlePhase::Command.allows(BattlePhase::Flee));
        assert!(BattlePhase::Flee.allows(BattlePhase::Advance));
        assert!(!BattlePhase::Victory.allows(BattlePhase::Command));
        assert!(!BattlePhase::Defeat.allows(BattlePhase::Advance));
    }

    #[test]
    fn enemy_target_selection_handles_single_wrap_cancel_and_no_target() {
        let actors = vec![
            actor(BattleSide::Party, 0, 4, 10),
            actor(BattleSide::Enemy, 0, 4, 0),
            actor(BattleSide::Enemy, 1, 4, 10),
            actor(BattleSide::Enemy, 2, 4, 10),
        ];
        let mut selector = TargetSelector::new(TargetGroup::Enemy, &actors, false).unwrap();
        assert_eq!(selector.selected(), CombatantKey::enemy(1));
        selector.navigate(-1);
        assert_eq!(selector.selected(), CombatantKey::enemy(2));
        selector.navigate(1);
        assert_eq!(selector.selected(), CombatantKey::enemy(1));
        assert!(TargetSelector::new(TargetGroup::Enemy, &actors[..2], false).is_none());

        let one = vec![actor(BattleSide::Enemy, 0, 1, 1)];
        let mut single = TargetSelector::new(TargetGroup::Enemy, &one, false).unwrap();
        single.navigate(1);
        assert_eq!(single.selected(), CombatantKey::enemy(0));
        // Cancellation is represented by dropping the selector and returning to Command.
        assert_eq!(BattlePhase::Command, BattlePhase::Command);
    }

    #[test]
    fn ally_targeting_distinguishes_living_and_ko_eligibility() {
        let actors = vec![
            actor(BattleSide::Party, 0, 3, 10),
            actor(BattleSide::Party, 1, 2, 0),
            actor(BattleSide::Enemy, 0, 1, 10),
        ];
        assert_eq!(
            TargetSelector::new(TargetGroup::Ally, &actors, false)
                .unwrap()
                .selected(),
            CombatantKey::party(0)
        );
        assert_eq!(
            TargetSelector::new(TargetGroup::Ally, &actors, true)
                .unwrap()
                .selected(),
            CombatantKey::party(1)
        );
    }

    #[test]
    fn turn_order_uses_dexterity_then_party_then_authored_order() {
        let actors = vec![
            actor(BattleSide::Enemy, 0, 8, 10),
            actor(BattleSide::Party, 0, 8, 10),
            actor(BattleSide::Party, 1, 12, 10),
            actor(BattleSide::Enemy, 1, 99, 0),
        ];
        assert_eq!(
            calculate_turn_order(&actors),
            vec![
                CombatantKey::party(1),
                CombatantKey::party(0),
                CombatantKey::enemy(0)
            ]
        );
    }

    #[test]
    fn hit_chance_clamps_at_both_boundaries_and_seeded_rolls_repeat() {
        assert_eq!(physical_hit_chance(100, 1), 0.95);
        assert_eq!(physical_hit_chance(1, 100), 0.05);
        assert_eq!(physical_hit_chance(20, 20), 0.70);
        let mut left = GameplayRng::from_seed(77);
        let mut right = GameplayRng::from_seed(77);
        let left = (0..16)
            .map(|_| roll_succeeds(&mut left, 0.7))
            .collect::<Vec<_>>();
        let right = (0..16)
            .map(|_| roll_succeeds(&mut right, 0.7))
            .collect::<Vec<_>>();
        assert_eq!(left, right);
    }

    #[test]
    fn flee_rate_uses_rogue_dex_caps_and_boss_or_failure_cost_rules() {
        let balance = BalanceData::default();
        let plain = state_with(vec![
            actor(BattleSide::Party, 0, 8, 10),
            actor(BattleSide::Enemy, 0, 4, 10),
        ]);
        assert_eq!(flee_chance(&plain, &balance), 0.30);

        let mut rogue = actor(BattleSide::Party, 0, 35, 10);
        rogue.class_id = "rogue".to_owned();
        let guaranteed = state_with(vec![rogue, actor(BattleSide::Enemy, 0, 4, 10)]);
        assert_eq!(flee_chance(&guaranteed, &balance), 1.0);

        let mut rng = GameplayRng::from_seed(5);
        let before = rng.state();
        assert_eq!(roll_flee(false, 1.0, &mut rng), FleeOutcome::Success);
        assert_eq!(
            rng.state(),
            before,
            "guaranteed flee consumes no random roll"
        );
        assert_eq!(roll_flee(true, 1.0, &mut rng), FleeOutcome::Blocked);
        assert_eq!(
            rng.state(),
            before,
            "boss restriction consumes no random roll"
        );

        assert_eq!(
            phase_after_flee_confirmation(FleeOutcome::Failed),
            Some(BattlePhase::Advance)
        );
        assert_eq!(
            phase_after_flee_confirmation(FleeOutcome::Blocked),
            Some(BattlePhase::Advance)
        );
        assert_eq!(phase_after_flee_confirmation(FleeOutcome::Success), None);
    }

    #[test]
    fn physical_damage_applies_attack_defense_rows_minimum_and_health_cap() {
        let mut front = actor(BattleSide::Party, 0, 1, 99);
        front.attack = 20;
        let mut enemy = actor(BattleSide::Enemy, 0, 1, 50);
        enemy.defense = 6;
        assert_eq!(physical_damage(&front, &enemy), 14);
        front.row = PartyRow::Back;
        assert_eq!(physical_damage(&front, &enemy), 7);
        enemy.health = 3;
        assert_eq!(physical_damage(&front, &enemy), 3);
        front.attack = 1;
        enemy.defense = 999;
        enemy.health = 50;
        assert_eq!(physical_damage(&front, &enemy), 1);
        let mut back_party = actor(BattleSide::Party, 1, 1, 50);
        back_party.row = PartyRow::Back;
        let enemy_attacker = actor(BattleSide::Enemy, 0, 1, 50);
        assert_eq!(physical_damage(&enemy_attacker, &back_party), 3);
    }

    #[test]
    fn damage_clamps_to_zero_and_turn_advance_skips_ko_actors() {
        let mut target = actor(BattleSide::Party, 0, 1, 4);
        assert_eq!(target.apply_damage(99), 4);
        assert!(!target.is_alive());

        let mut state = BattleState {
            phase: BattlePhase::Advance,
            combatants: vec![
                target,
                actor(BattleSide::Party, 1, 3, 10),
                actor(BattleSide::Enemy, 0, 2, 10),
            ],
            turn_order: vec![
                CombatantKey::party(0),
                CombatantKey::party(1),
                CombatantKey::enemy(0),
            ],
            active_turn: 0,
            command_index: 0,
            target: None,
            message: String::new(),
            transcript: Vec::new(),
            flee_outcome: None,
        };
        state.skip_knocked_out();
        assert_eq!(state.active_key(), Some(CombatantKey::party(1)));
    }

    #[test]
    fn victory_and_defeat_require_every_member_of_one_side_to_be_ko() {
        let mut state = BattleState {
            phase: BattlePhase::Resolve,
            combatants: vec![
                actor(BattleSide::Party, 0, 2, 10),
                actor(BattleSide::Enemy, 0, 1, 0),
                actor(BattleSide::Enemy, 1, 1, 1),
            ],
            turn_order: vec![],
            active_turn: 0,
            command_index: 0,
            target: None,
            message: String::new(),
            transcript: vec![],
            flee_outcome: None,
        };
        state.assess_result();
        assert_eq!(state.phase, BattlePhase::Advance);
        state.combatants[2].health = 0;
        state.assess_result();
        assert_eq!(state.phase, BattlePhase::Victory);
        state.combatants[0].health = 0;
        state.combatants[2].health = 1;
        state.assess_result();
        assert_eq!(state.phase, BattlePhase::Defeat);
    }

    #[test]
    fn deterministic_basic_battle_transcript_is_stable() {
        fn replay(seed: u64) -> Vec<String> {
            let mut state = BattleState {
                phase: BattlePhase::Command,
                combatants: vec![
                    actor(BattleSide::Party, 0, 8, 40),
                    actor(BattleSide::Enemy, 0, 4, 16),
                ],
                turn_order: vec![CombatantKey::party(0), CombatantKey::enemy(0)],
                active_turn: 0,
                command_index: 0,
                target: None,
                message: String::new(),
                transcript: vec!["START fixture".to_owned()],
                flee_outcome: None,
            };
            state.combatants[0].attack = 14;
            state.combatants[1].defense = 2;
            let mut rng = GameplayRng::from_seed(seed);
            while !state.all_defeated(BattleSide::Enemy) {
                if state.active_key() == Some(CombatantKey::party(0)) {
                    state.resolve_physical(
                        CombatantKey::party(0),
                        CombatantKey::enemy(0),
                        &mut rng,
                    );
                } else {
                    state.resolve_enemy_action(&mut rng);
                }
                state.assess_result();
                if state.phase == BattlePhase::Advance {
                    state.advance(&mut rng);
                }
            }
            state.transcript
        }
        let transcript = replay(9);
        assert_eq!(transcript, replay(9));
        assert_eq!(transcript.last().map(String::as_str), Some("VICTORY"));
    }
}
