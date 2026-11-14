//! Battle input: the per-frame system that drives every phase, and the helpers only it uses.

use super::*;

#[derive(SystemParam)]
pub(super) struct BattleInputContext<'w, 's> {
    reads: BattleInputReads<'w>,
    game: Option<ResMut<'w, GameState>>,
    deck: Option<ResMut<'w, deck::BattleDeck>>,
    entry: Option<Res<'w, BattleEntry>>,
    reward_prelude: Option<ResMut<'w, super::super::reward_modal::BattleRewardPrelude>>,
    reward_screen: Option<ResMut<'w, super::super::reward_modal::BattleRewardScreen>>,
    pointer: Option<Res<'w, crate::gameplay_canvas::pointer::CanvasPointer>>,
    reward_buttons: Query<
        'w,
        's,
        (&'static ComputedNode, &'static UiGlobalTransform),
        With<super::super::reward_modal::BattleRewardContinue>,
    >,
}

/// The read-only resources every battle phase may consult.
#[derive(SystemParam)]
pub(super) struct BattleInputReads<'w> {
    balances: Res<'w, Assets<BalanceData>>,
    active: Res<'w, ActiveNewGameInputs>,
    catalog: Res<'w, FieldMenuCatalog>,
    time: Res<'w, Time>,
    options: Option<Res<'w, PlayerOptions>>,
}

/// One frame's input and settings, read once before the phase handlers run.
struct PhaseInput<'a> {
    actions: &'a ActionState,
    balances: &'a Assets<BalanceData>,
    active: &'a ActiveNewGameInputs,
    catalog: &'a FieldMenuCatalog,
    options: Option<&'a PlayerOptions>,
    now: std::time::Duration,
    pause: std::time::Duration,
    auto_continue: bool,
    auto_enemy_turns: bool,
    skip_victory_screen: bool,
    pointer_confirm: bool,
}

impl<'a> PhaseInput<'a> {
    fn new(
        actions: &'a ActionState,
        reads: &'a BattleInputReads,
        state: &BattleState,
        pointer_confirm: bool,
    ) -> Self {
        let balances: &Assets<BalanceData> = &reads.balances;
        let active: &ActiveNewGameInputs = &reads.active;
        let catalog: &FieldMenuCatalog = &reads.catalog;
        let now = reads.time.elapsed();
        let auto_continue = state.auto_continue_at.is_some_and(|at| now >= at);
        let options = reads.options.as_deref();
        let (auto_enemy_turns, pause, skip_victory_screen) =
            options.map_or((false, TurnPause::default().duration(), false), |options| {
                (
                    options.auto_enemy_turns,
                    options.turn_pause.duration(),
                    options.skip_victory_screen,
                )
            });
        Self {
            actions,
            balances,
            active,
            catalog,
            options,
            now,
            pause,
            auto_continue,
            auto_enemy_turns,
            skip_victory_screen,
            pointer_confirm,
        }
    }
}

pub(super) fn handle_battle_input(
    mut commands: Commands,
    actions: Res<ActionState>,
    mut context: BattleInputContext,
    state: Option<ResMut<BattleState>>,
    mut transitions: MessageWriter<AppStateTransitionRequest>,
    mut timings: Option<ResMut<crate::frame_timing::FrameTimings>>,
    mut menu_sfx: MenuSfx,
) {
    let _measurement = timings
        .as_deref_mut()
        .map(|timings| timings.measure("battle.input_and_resolution"));
    let Some(entry) = context.entry.as_deref() else {
        return;
    };
    let Some(mut game) = context.game.take() else {
        return;
    };
    let Some(mut state) = state else {
        return;
    };
    let mut deck = context.deck.take();
    let mut fallback_deck = deck::BattleDeck::default();
    let deck = frame_deck(&actions, &mut deck, &mut fallback_deck);
    let pointer_confirm = deck.take_pointer_confirm();
    let input = PhaseInput::new(&actions, &context.reads, &state, pointer_confirm);
    let (game, state, sfx) = (&mut game, &mut state, &mut menu_sfx);
    match state.phase {
        BattlePhase::Start | BattlePhase::Advance => start_or_advance_phase(state, game, &input),
        BattlePhase::Command
            if !state.auto_suspended && active_auto_command(state, game).is_some() =>
        {
            auto_command_phase(state, game, &input, sfx);
        }
        BattlePhase::Command => command_phase(state, game, deck, &input, sfx),
        BattlePhase::Ability => ability_phase(state, game, deck, &input, sfx),
        BattlePhase::Item => item_phase(state, &input),
        BattlePhase::Target => target_phase(state, game, &input),
        BattlePhase::Resolve if actions.just_pressed(AppAction::Confirm) || input.auto_continue => {
            resolve_phase(state);
        }
        BattlePhase::Victory
            if actions.just_pressed(AppAction::Confirm) || input.skip_victory_screen =>
        {
            victory_phase(&mut commands, state, game, entry, &input, &mut transitions);
        }
        // Confirm or a click during the victory sting reveals the loot now. The modal's own
        // Continue needs a fresh press, so this one press cannot also dismiss it.
        BattlePhase::Rewards if reveal_loot_requested(&actions, &context) => {
            if let Some(prelude) = context.reward_prelude.as_deref_mut() {
                prelude.skip();
            }
        }
        BattlePhase::Rewards if continue_rewards_requested(&actions, &context) => {
            continue_rewards(
                &mut commands,
                state,
                game,
                entry,
                &mut context.reward_screen,
                &mut transitions,
            );
        }
        BattlePhase::Defeat => {
            transitions.write(AppStateTransitionRequest::new(AppState::GameOver));
        }
        BattlePhase::Flee if actions.just_pressed(AppAction::Confirm) || input.auto_continue => {
            flee_phase(&mut commands, state, game, entry, &mut transitions);
        }
        _ => {}
    }
}

/// Marks the deck changed on navigation frames, then hands out a reference that does not.
fn frame_deck<'a>(
    actions: &ActionState,
    deck: &'a mut Option<ResMut<deck::BattleDeck>>,
    fallback_deck: &'a mut deck::BattleDeck,
) -> &'a mut deck::BattleDeck {
    // Navigation changes the view; idle frames must not rebuild its entities.
    let view_input = actions.menu_navigation().is_some()
        || actions.menu_navigation_horizontal().is_some()
        || actions.just_pressed(AppAction::Confirm)
        || actions.just_pressed(AppAction::Back);
    if view_input && let Some(deck) = deck.as_mut() {
        deck.set_changed();
    }
    deck.as_mut()
        .map(|deck| deck.bypass_change_detection())
        .unwrap_or(fallback_deck)
}

fn start_or_advance_phase(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    input: &PhaseInput,
) {
    if state.phase == BattlePhase::Start {
        state.begin_active_turn(game.rng_mut());
    } else {
        state.advance(game.rng_mut());
    }
    // An enemy acts the moment its turn begins; with auto enemy turns on, its result
    // moves on by itself like an auto party member's.
    let enemy_acted = state.phase == BattlePhase::Resolve
        && state
            .active()
            .is_some_and(|active| active.key.side == BattleSide::Enemy);
    if enemy_acted && input.auto_enemy_turns {
        state.auto_continue_at = Some(input.now + input.pause);
    }
}

fn auto_command_phase(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    input: &PhaseInput,
    menu_sfx: &mut MenuSfx,
) {
    let command = active_auto_command(state, game)
        .expect("dispatcher match guard calls this only when active_auto_command is Some");
    run_auto_turn(
        state,
        command,
        game,
        input.catalog,
        input.active,
        input.balances,
        menu_sfx,
    );
    if matches!(state.phase, BattlePhase::Resolve | BattlePhase::Flee) {
        state.auto_continue_at = Some(input.now + input.pause);
    }
}

fn command_phase(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    deck: &mut deck::BattleDeck,
    input: &PhaseInput,
    menu_sfx: &mut MenuSfx,
) {
    let actions = input.actions;
    if actions.just_pressed(AppAction::Back) {
        attempt_flee(state, input.active, input.balances, game, menu_sfx);
        return;
    }
    if let Some(movement) = actions
        .menu_navigation_horizontal()
        .or_else(|| actions.menu_navigation())
    {
        state.command_index = wrap_index(state.command_index, movement, COMMANDS.len());
        menu_sfx.hover();
    }
    if actions.just_pressed(AppAction::Confirm) || input.pointer_confirm {
        confirm_command(state, game, deck, input, menu_sfx);
    }
}

fn confirm_command(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    deck: &mut deck::BattleDeck,
    input: &PhaseInput,
    menu_sfx: &mut MenuSfx,
) {
    let options = input.options;
    let command = COMMANDS[state.command_index];
    if !state.command_available(command) {
        state.message = match command {
            BattleCommand::Spell => "No usable abilities.".to_owned(),
            BattleCommand::Item => "No battle items available.".to_owned(),
            BattleCommand::Run => "Can't escape from a boss!".to_owned(),
            BattleCommand::Attack => "That action is unavailable.".to_owned(),
        };
        menu_sfx.blocked();
        return;
    }
    menu_sfx.confirm();
    // A new command must not inherit the previous spell/item's target return path.
    state.pending_ability = None;
    state.pending_item = None;
    match command {
        BattleCommand::Attack => {
            state.target = TargetSelector::new(TargetGroup::Enemy, &state.combatants, false);
            if state.target.is_some() {
                debug_assert!(state.phase.allows(BattlePhase::Target));
                state.phase = BattlePhase::Target;
                state.message = cancel_prompt(options, "Choose a living enemy.");
            } else {
                state.assess_result();
            }
        }
        BattleCommand::Run => {
            attempt_flee(state, input.active, input.balances, game, menu_sfx);
        }
        BattleCommand::Spell => {
            state.ability_index = 0;
            state.pending_ability = None;
            state.phase = BattlePhase::Ability;
            deck.open(state);
            state.message = spell_groups_hint(options);
        }
        BattleCommand::Item => {
            state.item_index = 0;
            state.pending_item = None;
            state.phase = BattlePhase::Item;
            state.message = cancel_prompt(options, "Choose an item.");
        }
    }
}

fn ability_phase(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    deck: &mut deck::BattleDeck,
    input: &PhaseInput,
    menu_sfx: &mut MenuSfx,
) {
    let (actions, options) = (input.actions, input.options);
    if cancel_pressed(actions) {
        if deck.list_focused {
            deck.list_focused = false;
            state.message = spell_groups_hint(options);
        } else {
            state.phase = BattlePhase::Command;
            state.message = "Action cancelled.".to_owned();
        }
    } else if !deck.list_focused {
        if let Some(delta) = actions.menu_navigation() {
            deck.move_group(state, delta);
            menu_sfx.hover();
        }
        if confirm_pressed(actions) {
            deck.list_focused = true;
            state.message = spell_list_hint(options);
            menu_sfx.confirm();
        }
    } else {
        if let Some(delta) = actions.menu_navigation() {
            deck.move_spell(state, delta);
            menu_sfx.hover();
        }
        if (actions.just_pressed(AppAction::Confirm) || input.pointer_confirm)
            && let Some(&ability_index) = state.battle_ability_indices().get(state.ability_index)
        {
            begin_ability_targeting(state, ability_index, game.rng_mut(), options);
        }
    }
}

fn item_phase(state: &mut ResMut<BattleState>, input: &PhaseInput) {
    let actions = input.actions;
    if cancel_pressed(actions) {
        state.phase = BattlePhase::Command;
        state.message = "Action cancelled.".to_owned();
    } else {
        if let Some(movement) = actions.menu_navigation() {
            state.item_index = wrap_index(state.item_index, movement, state.item_choices.len());
        }
        if (confirm_pressed(actions) || input.pointer_confirm)
            && let Some(choice) = state.item_choices.get(state.item_index).cloned()
        {
            begin_item_targeting(state, &choice.id, input.catalog, input.options);
        }
    }
}

fn target_phase(state: &mut ResMut<BattleState>, game: &mut ResMut<GameState>, input: &PhaseInput) {
    let actions = input.actions;
    if actions.just_pressed(AppAction::Back) {
        state.target = None;
        state.phase = if state.pending_ability.is_some() {
            BattlePhase::Ability
        } else if state.pending_item.is_some() {
            BattlePhase::Item
        } else {
            BattlePhase::Command
        };
        state.message = "Action cancelled.".to_owned();
    } else {
        // Enemies stand in a row, so Left/Right cycle the same pool Up/Down do.
        if let Some(movement) = target_navigation(actions)
            && let Some(target) = state.target.as_mut()
        {
            target.navigate(movement);
        }
        if actions.just_pressed(AppAction::Confirm) {
            confirm_target(state, game, input.catalog);
        }
    }
}

fn confirm_target(
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    catalog: &FieldMenuCatalog,
) {
    let attacker = state.active_key();
    let target = state.target.as_ref().map(TargetSelector::selected);
    state.target = None;
    if let (Some(attacker), Some(target)) = (attacker, target) {
        if let Some(ability) = state.pending_ability.take() {
            resolve_selected_ability(state, attacker, ability, &[target], game.rng_mut());
        } else if let Some(item_id) = state.pending_item.take() {
            resolve_selected_item(
                state,
                attacker,
                &item_id,
                target,
                catalog,
                game.repository_mut(),
            );
        } else {
            state.resolve_physical(attacker, target, game.rng_mut());
        }
    }
}

fn resolve_phase(state: &mut ResMut<BattleState>) {
    state.auto_continue_at = None;
    state.assess_result();
}

fn victory_phase(
    commands: &mut Commands,
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    entry: &BattleEntry,
    input: &PhaseInput,
    transitions: &mut MessageWriter<AppStateTransitionRequest>,
) {
    let Some(balance) = input.active.balance(input.balances) else {
        state.message = "Battle balance is still loading.".to_owned();
        return;
    };
    match apply_rewards(
        state,
        game,
        input.catalog,
        balance,
        entry.boss_completion_flag.as_deref(),
        &entry.victory_flags,
    ) {
        Ok(rewards) => {
            debug_assert_eq!(state.rewards.as_ref(), Some(&rewards));
            if input.skip_victory_screen {
                // The spoils are already banked; go straight back to the map.
                restore_world(commands, game, entry);
                transitions.write(AppStateTransitionRequest::new(AppState::World));
            } else {
                state.phase = BattlePhase::Rewards;
                commands.insert_resource(super::super::reward_modal::BattleRewardScreen::Loot);
            }
        }
        Err(error) => state.message = format!("Could not apply rewards: {error}"),
    }
}

fn reveal_loot_requested(actions: &ActionState, context: &BattleInputContext) -> bool {
    context.reward_prelude.is_some()
        && (actions.just_pressed(AppAction::Confirm)
            || context
                .pointer
                .as_ref()
                .is_some_and(|pointer| pointer.just_clicked()))
}

fn continue_rewards_requested(actions: &ActionState, context: &BattleInputContext) -> bool {
    context.reward_prelude.is_none()
        && (actions.just_pressed(AppAction::Confirm)
            || context.pointer.as_ref().is_some_and(|pointer| {
                pointer.just_clicked()
                    && context
                        .reward_buttons
                        .iter()
                        .any(|(node, transform)| pointer.is_over(node, transform))
            }))
}

fn continue_rewards(
    commands: &mut Commands,
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    entry: &BattleEntry,
    reward_screen: &mut Option<ResMut<super::super::reward_modal::BattleRewardScreen>>,
    transitions: &mut MessageWriter<AppStateTransitionRequest>,
) {
    if let (Some(screen), Some(rewards)) = (reward_screen.as_deref_mut(), state.rewards.as_ref())
        && screen.advance(rewards)
    {
        return;
    }
    restore_world(commands, game, entry);
    transitions.write(AppStateTransitionRequest::new(AppState::World));
}

fn flee_phase(
    commands: &mut Commands,
    state: &mut ResMut<BattleState>,
    game: &mut ResMut<GameState>,
    entry: &BattleEntry,
    transitions: &mut MessageWriter<AppStateTransitionRequest>,
) {
    state.auto_continue_at = None;
    let outcome = state.flee_outcome.unwrap_or(FleeOutcome::Failed);
    if phase_after_flee_confirmation(outcome).is_none() {
        restore_world(commands, game, entry);
        transitions.write(AppStateTransitionRequest::new(AppState::World));
    } else {
        state.phase = phase_after_flee_confirmation(outcome)
            .expect("failed and blocked flee consume the active turn");
    }
}

/// The battle message hints: a space between key and label, a dot between segments.
const BATTLE_HINTS: HintStyle = HintStyle {
    key_gap: " ",
    segment_gap: " · ",
    pair: "/",
    either: " / ",
    label_first: false,
};

fn battle_hint(options: Option<&PlayerOptions>, segments: &[HintSegment<'_>]) -> String {
    match options {
        Some(options) => key_hint(&options.bindings, Device::Keyboard, &BATTLE_HINTS, segments),
        None => key_hint(
            &InputBindings::default(),
            Device::Keyboard,
            &BATTLE_HINTS,
            segments,
        ),
    }
}

const BATTLE_UP_DOWN: HintKey = HintKey::Pair(AppAction::Up, AppAction::Down);

/// `Choose a target. ESC cancels.`: a prompt naming the key that backs out of the choice.
fn cancel_prompt(options: Option<&PlayerOptions>, prompt: &str) -> String {
    let back = [HintKey::Action(AppAction::Back)];
    let key = match options {
        Some(options) => hint_keys_text(&options.bindings, Device::Keyboard, &BATTLE_HINTS, &back),
        None => hint_keys_text(
            &InputBindings::default(),
            Device::Keyboard,
            &BATTLE_HINTS,
            &back,
        ),
    };
    format!("{prompt} {key} cancels.")
}

/// The spell menu's group column. Groups move on Up and Down, Right (or Confirm) enters the
/// list and Back leaves, all through the action table.
fn spell_groups_hint(options: Option<&PlayerOptions>) -> String {
    battle_hint(
        options,
        &[
            (&[BATTLE_UP_DOWN], "group"),
            (&[HintKey::Action(AppAction::Right)], "spells"),
            (&[HintKey::Action(AppAction::Back)], "commands"),
        ],
    )
}

/// The spell menu's list column. Left (or Back) returns to the groups.
fn spell_list_hint(options: Option<&PlayerOptions>) -> String {
    battle_hint(
        options,
        &[
            (&[BATTLE_UP_DOWN], "spell"),
            (&[HintKey::Action(AppAction::Confirm)], "cast"),
            (&[HintKey::Action(AppAction::Left)], "groups"),
        ],
    )
}

fn begin_item_targeting(
    state: &mut BattleState,
    item_id: &str,
    catalog: &FieldMenuCatalog,
    options: Option<&PlayerOptions>,
) {
    let Some(item) = catalog.item(item_id).and_then(battle_item) else {
        state.message = "That item cannot be used in battle.".to_owned();
        return;
    };
    let plan = item_target_plan(item);
    state.target = TargetSelector::new(plan.group, &state.combatants, plan.ko_eligible);
    if state.target.is_some() {
        state.pending_item = Some(item_id.to_owned());
        state.phase = BattlePhase::Target;
        state.message = cancel_prompt(options, "Choose a target.");
    } else {
        state.message = "No valid target.".to_owned();
    }
}

fn resolve_selected_item(
    state: &mut BattleState,
    source: CombatantKey,
    item_id: &str,
    target: CombatantKey,
    catalog: &FieldMenuCatalog,
    repository: &mut crate::runtime_repository::RuntimeRepository,
) {
    let Some(item) = catalog.item(item_id).and_then(battle_item) else {
        state.phase = BattlePhase::Item;
        state.message = "That item cannot be used in battle.".to_owned();
        return;
    };
    match resolve_item(state, source, item, target, repository) {
        Ok(_) => {
            if let Some(choice) = state
                .item_choices
                .iter_mut()
                .find(|choice| choice.id == item_id)
            {
                choice.quantity = choice.quantity.saturating_sub(1);
            }
            state.item_choices.retain(|choice| choice.quantity > 0);
            state.item_index = state
                .item_index
                .min(state.item_choices.len().saturating_sub(1));
        }
        Err(error) => {
            state.phase = BattlePhase::Item;
            state.message = match error {
                ItemUseError::InvalidTarget => "No valid target.".to_owned(),
                ItemUseError::Unavailable => "That item is no longer available.".to_owned(),
                ItemUseError::UnknownSource | ItemUseError::Unsupported => {
                    "That item cannot be used here.".to_owned()
                }
            };
        }
    }
}

fn begin_ability_targeting(
    state: &mut BattleState,
    ability_index: usize,
    rng: &mut crate::gameplay_rng::GameplayRng,
    options: Option<&PlayerOptions>,
) {
    let Some(caster) = state.active_key() else {
        return;
    };
    let Some(ability) = state
        .actor(caster)
        .and_then(|actor| actor.abilities.get(ability_index))
        .cloned()
    else {
        return;
    };
    if state
        .actor(caster)
        .is_none_or(|actor| actor.mana < ability.mp_cost)
    {
        state.message = format!("Not enough MP for {}.", ability.name);
        return;
    }
    match target_plan(&ability) {
        AbilityTargetPlan::Select { group, ko_eligible } => {
            state.target = TargetSelector::new(group, &state.combatants, ko_eligible);
            if state.target.is_some() {
                state.pending_ability = Some(ability_index);
                state.phase = BattlePhase::Target;
                state.message = cancel_prompt(options, "Choose a target.");
            } else {
                state.message = "No valid target.".to_owned();
            }
        }
        AbilityTargetPlan::All { side, ko_eligible } => {
            let targets = state
                .combatants
                .iter()
                .filter(|actor| actor.key.side == side && actor.is_alive() != ko_eligible)
                .map(|actor| actor.key)
                .collect::<Vec<_>>();
            resolve_selected_ability(state, caster, ability_index, &targets, rng);
        }
        AbilityTargetPlan::SelfTarget => {
            resolve_selected_ability(state, caster, ability_index, &[caster], rng);
        }
    }
}

fn resolve_selected_ability(
    state: &mut BattleState,
    caster: CombatantKey,
    ability_index: usize,
    targets: &[CombatantKey],
    rng: &mut crate::gameplay_rng::GameplayRng,
) {
    if let Err(error) = resolve_ability(state, caster, ability_index, targets, rng) {
        state.phase = BattlePhase::Ability;
        state.message = match error {
            AbilityError::InsufficientMana => "Not enough MP.".to_owned(),
            AbilityError::Silenced => "Silence prevents spellcasting.".to_owned(),
            AbilityError::InvalidTarget => "No valid target.".to_owned(),
            AbilityError::UnknownCaster
            | AbilityError::UnknownAbility
            | AbilityError::Unsupported => "That ability cannot be used here.".to_owned(),
        };
    }
}

/// The default command of the member whose turn it is, if the player set them to Auto.
fn active_auto_command(state: &BattleState, game: &GameState) -> Option<AutoCommand> {
    let active = state.active()?;
    if active.key.side != BattleSide::Party {
        return None;
    }
    game.party().member(&active.id)?.tactics().auto_command()
}

/// Plays one auto member's turn through the same resolvers a manual command uses.
///
/// A plan the resolvers refuse falls back to a plain attack, and if even that cannot land the
/// turn is handed to the player rather than retried every frame.
fn run_auto_turn(
    state: &mut BattleState,
    command: AutoCommand,
    game: &mut GameState,
    catalog: &FieldMenuCatalog,
    active: &ActiveNewGameInputs,
    balances: &Assets<BalanceData>,
    menu_sfx: &mut MenuSfx,
) {
    let Some(actor) = state.active_key() else {
        return;
    };
    let name = state
        .active()
        .map(|active| active.name.clone())
        .unwrap_or_default();
    let items = auto_items(&state.item_choices, game.repository(), |id| {
        catalog.item(id)
    });
    let plan = plan_auto_action(state, actor, command, &items, game.rng_mut());
    state.transcript.push(format!("AUTO {name} {plan:?}"));
    match plan {
        Some(AutoAction::Attack(target)) => {
            state.resolve_physical(actor, target, game.rng_mut());
        }
        Some(AutoAction::Ability { index, targets }) => {
            resolve_selected_ability(state, actor, index, &targets, game.rng_mut());
        }
        Some(AutoAction::Item { id, target }) => {
            resolve_selected_item(state, actor, &id, target, catalog, game.repository_mut());
        }
        Some(AutoAction::Flee) => {
            attempt_flee(state, active, balances, game, menu_sfx);
            return;
        }
        None => {}
    }
    let unresolved = |state: &BattleState| {
        matches!(
            state.phase,
            BattlePhase::Command | BattlePhase::Ability | BattlePhase::Item
        )
    };
    if unresolved(state)
        && let Some(target) = state
            .combatants
            .iter()
            .filter(|enemy| enemy.key.side == BattleSide::Enemy && enemy.is_alive())
            .min_by_key(|enemy| enemy.health)
            .map(|enemy| enemy.key)
    {
        state.phase = BattlePhase::Command;
        state.resolve_physical(actor, target, game.rng_mut());
    }
    if unresolved(state) {
        state.phase = BattlePhase::Command;
        state.auto_suspended = true;
        state.message = format!("{name} awaits your orders.");
    } else if state.phase != BattlePhase::Victory {
        state.message = format!("[Auto] {}", state.message);
    }
}

fn attempt_flee(
    state: &mut BattleState,
    active: &ActiveNewGameInputs,
    balances: &Assets<BalanceData>,
    game: &mut GameState,
    menu_sfx: &mut MenuSfx,
) {
    let Some(balance) = active.balance(balances) else {
        state.message = "Battle balance is still loading.".to_owned();
        menu_sfx.blocked();
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
    // Getting away plays the escape cue; a failed or boss-blocked attempt plays the refusal.
    match outcome {
        FleeOutcome::Success => menu_sfx.play(cue::FLEE),
        FleeOutcome::Failed | FleeOutcome::Blocked => menu_sfx.blocked(),
    }
    state.transcript.push(format!("FLEE {outcome:?}"));
}

fn restore_world(commands: &mut Commands, game: &mut GameState, entry: &BattleEntry) {
    if restore_pre_battle_context(game, &entry.return_context).is_ok() {
        commands.insert_resource(WorldBgmFadeInRequest);
        commands.insert_resource(WorldEncounterRestore {
            map_id: entry.return_context.map_id.clone(),
            enemies: entry.return_context.world_enemies.clone(),
        });
    }
}
