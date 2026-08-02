from pathlib import Path


def replace_once(path: str, old: str, new: str) -> None:
    target = Path(path)
    text = target.read_text()
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"expected one match in {path}, found {count}: {old[:140]!r}")
    target.write_text(text.replace(old, new, 1))


def replace_between(path: str, start: str, end: str, replacement: str) -> None:
    target = Path(path)
    text = target.read_text()
    start_index = text.index(start)
    end_index = text.index(end, start_index)
    target.write_text(text[:start_index] + replacement + text[end_index:])


# --- keymap model ---------------------------------------------------------
path = Path("src/keybindings.rs")
text = path.read_text()

insert_after_action = '''}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyCodeSpec {
'''
model = '''}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ActionBindingState {
    Bound,
    Unbound,
    Disabled,
}

impl ActionBindingState {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Bound => "bound",
            Self::Unbound => "unbound",
            Self::Disabled => "disabled",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) enum InputContext {
    Main,
    Report,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ResolvedActionSource {
    Mandatory,
    Direct,
    Contextual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ResolvedAction {
    pub action: Action,
    pub source: ResolvedActionSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AliasCondition {
    Always,
    TargetUnbound,
}

impl AliasCondition {
    const fn label(self) -> &'static str {
        match self {
            Self::Always => "always",
            Self::TargetUnbound => "when target unbound",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ContextualAlias {
    name: String,
    context: InputContext,
    source: Action,
    target: Action,
    condition: AliasCondition,
}

impl ContextualAlias {
    pub(crate) fn display_label(&self) -> String {
        format!("{} → {} ({})", self.name, self.target.config_name(), self.condition.label())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum KeyCodeSpec {
'''
if text.count(insert_after_action) != 1:
    raise SystemExit("action/model insertion anchor not found")
text = text.replace(insert_after_action, model, 1)

# Mandatory binding comparison helper.
text = text.replace(
    '''    fn to_config_string(self) -> String {
''',
    '''    fn mandatory_quit() -> Self {
        Self {
            ctrl: true,
            alt: false,
            shift: false,
            code: KeyCodeSpec::Char('c'),
        }
    }

    fn is_mandatory_quit(self) -> bool {
        self == Self::mandatory_quit()
    }

    fn to_config_string(self) -> String {
''',
    1,
)

replace_between(
    "src/keybindings.rs",
    "#[derive(Debug, Clone)]\npub(crate) struct Keymap",
    "#[derive(Debug, Clone, Serialize, Deserialize)]\nstruct KeymapConfig",
    '''#[derive(Debug, Clone)]
pub(crate) struct Keymap {
    bindings: HashMap<KeyBinding, Action>,
    disabled_actions: HashSet<Action>,
    contextual_aliases: Vec<ContextualAlias>,
}

impl Keymap {
    fn empty() -> Self {
        Self {
            bindings: HashMap::new(),
            disabled_actions: HashSet::new(),
            contextual_aliases: Vec::new(),
        }
    }

    fn bind(&mut self, key: KeyBinding, action: Action) {
        self.bindings.insert(key, action);
    }

    fn unbind_key(&mut self, key: &KeyBinding) {
        self.bindings.remove(key);
    }

    fn unbind_action(&mut self, action: Action) {
        self.bindings
            .retain(|_, current_action| *current_action != action);
    }

    fn disable_action(&mut self, action: Action) {
        self.unbind_action(action);
        self.disabled_actions.insert(action);
    }

    pub(crate) fn action_state(&self, action: Action) -> ActionBindingState {
        if self.disabled_actions.contains(&action) {
            ActionBindingState::Disabled
        } else if self.bindings.values().any(|mapped| *mapped == action) {
            ActionBindingState::Bound
        } else {
            ActionBindingState::Unbound
        }
    }

    pub(crate) fn action_for_key_event(&self, event: KeyEvent) -> Option<Action> {
        self.resolve_key_event(InputContext::Other, event)
            .map(|resolved| resolved.action)
    }

    pub(crate) fn mandatory_action_for_key_event(&self, event: KeyEvent) -> Option<Action> {
        let key = KeyBinding::from_key_event(event)?;
        key.is_mandatory_quit().then_some(Action::Quit)
    }

    pub(crate) fn resolve_key_event(
        &self,
        context: InputContext,
        event: KeyEvent,
    ) -> Option<ResolvedAction> {
        if let Some(action) = self.mandatory_action_for_key_event(event) {
            return Some(ResolvedAction {
                action,
                source: ResolvedActionSource::Mandatory,
            });
        }

        let key = KeyBinding::from_key_event(event)?;
        let source_action = self.bindings.get(&key).copied()?;
        if self.action_state(source_action) == ActionBindingState::Disabled {
            return None;
        }

        if let Some(alias) = self
            .contextual_aliases
            .iter()
            .find(|alias| alias.context == context && alias.source == source_action)
        {
            let target_state = self.action_state(alias.target);
            let applies = target_state != ActionBindingState::Disabled
                && match alias.condition {
                    AliasCondition::Always => true,
                    AliasCondition::TargetUnbound => target_state == ActionBindingState::Unbound,
                };
            if applies {
                return Some(ResolvedAction {
                    action: alias.target,
                    source: ResolvedActionSource::Contextual,
                });
            }
        }

        Some(ResolvedAction {
            action: source_action,
            source: ResolvedActionSource::Direct,
        })
    }

    pub(crate) fn keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        let mut keys: Vec<KeyBinding> = self
            .bindings
            .iter()
            .filter_map(|(key, mapped_action)| (*mapped_action == action).then_some(*key))
            .collect();
        keys.sort_by_key(|key| key.to_string());
        keys
    }

    pub(crate) fn mandatory_keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        if action == Action::Quit {
            vec![KeyBinding::mandatory_quit()]
        } else {
            Vec::new()
        }
    }

    pub(crate) fn aliases_for_action(&self, action: Action) -> Vec<ContextualAlias> {
        let mut aliases = self
            .contextual_aliases
            .iter()
            .filter(|alias| alias.target == action)
            .cloned()
            .collect::<Vec<_>>();
        aliases.sort_by(|left, right| left.name.cmp(&right.name));
        aliases
    }
}

''',
)

# Configuration fields for contextual policy.
text = text.replace(
    '''    #[serde(default)]
    unbind_actions: Vec<String>,
''',
    '''    #[serde(default)]
    unbind_actions: Vec<String>,
    #[serde(default = "default_true")]
    contextual_aliases_inherit: bool,
    #[serde(default)]
    contextual_aliases: BTreeMap<String, Option<String>>,
''',
    1,
)
text = text.replace(
    '''            unbind_actions: Vec::new(),
            day_start_mode: None,
''',
    '''            unbind_actions: Vec::new(),
            contextual_aliases_inherit: true,
            contextual_aliases: BTreeMap::new(),
            day_start_mode: None,
''',
    1,
)

# Default keymap: ctrl-c is mandatory, not a configurable direct binding.
text = text.replace("const DEFAULT_BINDINGS: [(&str, Action); 31]", "const DEFAULT_BINDINGS: [(&str, Action); 30]", 1)
text = text.replace('    ("ctrl-c", Action::Quit),\n', "", 1)

# Default alias definitions and loader.
default_aliases = r'''
#[derive(Debug, Clone, Copy)]
struct ContextualAliasDefinition {
    name: &'static str,
    context: InputContext,
    source: Action,
    target: Action,
    condition: AliasCondition,
}

const DEFAULT_CONTEXTUAL_ALIASES: [ContextualAliasDefinition; 4] = [
    ContextualAliasDefinition {
        name: "main.confirm",
        context: InputContext::Main,
        source: Action::Confirm,
        target: Action::OpenCategoryModal,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "main.cancel",
        context: InputContext::Main,
        source: Action::Cancel,
        target: Action::SwitchToNone,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "main.karma_today",
        context: InputContext::Main,
        source: Action::ReportToday,
        target: Action::Detach,
        condition: AliasCondition::TargetUnbound,
    },
    ContextualAliasDefinition {
        name: "report.detach",
        context: InputContext::Report,
        source: Action::Detach,
        target: Action::ReportToday,
        condition: AliasCondition::Always,
    },
];

fn contextual_alias_definition(name: &str) -> Option<ContextualAliasDefinition> {
    DEFAULT_CONTEXTUAL_ALIASES
        .iter()
        .copied()
        .find(|definition| definition.name == name)
}

fn default_contextual_aliases() -> Vec<ContextualAlias> {
    DEFAULT_CONTEXTUAL_ALIASES
        .iter()
        .map(|definition| ContextualAlias {
            name: definition.name.to_string(),
            context: definition.context,
            source: definition.source,
            target: definition.target,
            condition: definition.condition,
        })
        .collect()
}

fn parse_contextual_aliases(
    config: &KeymapConfig,
    path: &Path,
) -> Result<Vec<ContextualAlias>, String> {
    let mut aliases = if config.contextual_aliases_inherit {
        default_contextual_aliases()
    } else {
        Vec::new()
    };

    for (name, configured_target) in &config.contextual_aliases {
        let definition = contextual_alias_definition(name).ok_or_else(|| {
            let available = DEFAULT_CONTEXTUAL_ALIASES
                .iter()
                .map(|definition| definition.name)
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "Unknown contextual alias '{}' in {}. Available aliases: {available}",
                name,
                path.display()
            )
        })?;
        aliases.retain(|alias| alias.name != *name);
        let Some(target_name) = configured_target else {
            continue;
        };
        let target = Action::from_config_name(target_name).ok_or_else(|| {
            format!(
                "Unknown contextual target '{}' for '{}' in {}",
                target_name,
                name,
                path.display()
            )
        })?;
        aliases.push(ContextualAlias {
            name: name.clone(),
            context: definition.context,
            source: definition.source,
            target,
            condition: definition.condition,
        });
    }

    Ok(aliases)
}

'''
anchor = "pub(crate) fn default_keymap() -> Keymap {\n"
if text.count(anchor) != 1:
    raise SystemExit("default keymap anchor not found")
text = text.replace(anchor, default_aliases + anchor, 1)
text = text.replace(
    '''    for (raw_key, action) in DEFAULT_BINDINGS {
        if let Ok(key) = KeyBinding::parse(raw_key) {
            keymap.bind(key, action);
        }
    }
    keymap
''',
    '''    for (raw_key, action) in DEFAULT_BINDINGS {
        if let Ok(key) = KeyBinding::parse(raw_key) {
            keymap.bind(key, action);
        }
    }
    keymap.contextual_aliases = default_contextual_aliases();
    keymap
''',
    1,
)

# Load policy, reject mandatory key config, retain Disabled state regardless inherit.
text = text.replace(
    '''    let time_log_path = parse_time_log_path(&config, path)?;
    let mut unbound_actions = parse_unbound_actions(&config, path)?;
''',
    '''    let time_log_path = parse_time_log_path(&config, path)?;
    let contextual_aliases = parse_contextual_aliases(&config, path)?;
    let mut disabled_actions = parse_unbound_actions(&config, path)?;
''',
    1,
)
text = text.replace("                unbound_actions.remove(&action);", "                disabled_actions.remove(&action);", 1)
text = text.replace(
    '''        let parsed_action = match raw_action {
''',
    '''        if parsed_key.is_mandatory_quit() {
            return Err(format!(
                "Key 'ctrl-c' in {} is mandatory Quit policy and cannot be configured",
                path.display()
            ));
        }

        let parsed_action = match raw_action {
''',
    1,
)
text = text.replace(
    '''    if config.keymap_inherit {
        for action in overridden_actions {
            keymap.unbind_action(action);
        }

        for action in unbound_actions {
            keymap.unbind_action(action);
        }
    }
''',
    '''    if config.keymap_inherit {
        for action in overridden_actions {
            keymap.unbind_action(action);
        }
    }
''',
    1,
)
text = text.replace(
    '''    for (key, action) in parsed_overrides {
        if let Some(action) = action {
            keymap.bind(key, action);
        } else {
            keymap.unbind_key(&key);
        }
    }

    Ok(LoadedKeybindings {
''',
    '''    for (key, action) in parsed_overrides {
        if let Some(action) = action {
            keymap.bind(key, action);
        } else {
            keymap.unbind_key(&key);
        }
    }
    for action in disabled_actions {
        keymap.disable_action(action);
    }
    keymap.contextual_aliases = contextual_aliases;

    Ok(LoadedKeybindings {
''',
    1,
)

# Atlas editor can explicitly unbind without disabling.
insert_before = "pub(crate) fn set_time_log_path(\n"
set_unbound = r'''pub(crate) fn set_action_unbound(
    path: &Path,
    action: Action,
) -> Result<LoadedKeybindings, String> {
    let mut config = load_config_or_default(path)?;
    remove_action_keymap_entries(&mut config, action);
    remove_unbound_action_marker(&mut config, action);

    if config.keymap_inherit {
        for (raw_key, default_action) in DEFAULT_BINDINGS {
            if default_action == action {
                config.keymap.insert(raw_key.to_string(), None);
            }
        }
    }

    save_config(path, &config)?;
    load_keybindings(path)
}

'''
idx = text.index(insert_before)
text = text[:idx] + set_unbound + text[idx:]

# Add comprehensive keymap policy tests before final module close.
test_insert = r'''

    #[test]
    fn mandatory_ctrl_c_is_separate_from_configured_quit_state() {
        let path = unique_path("strata_keymap_mandatory_quit");
        fs::write(&path, r#"{"unbind_actions":["quit"]}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(keymap.action_state(Action::Quit), ActionBindingState::Disabled);
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let resolved = keymap.resolve_key_event(InputContext::Main, ctrl_c).unwrap();
        assert_eq!(resolved.action, Action::Quit);
        assert_eq!(resolved.source, ResolvedActionSource::Mandatory);
        fs::remove_file(path).ok();
    }

    #[test]
    fn configuring_mandatory_ctrl_c_is_rejected() {
        let path = unique_path("strata_keymap_mandatory_conflict");
        fs::write(&path, r#"{"keymap":{"ctrl-c":"open_karma_popup"}}"#).unwrap();
        let error = load_keymap_for_test(&path).unwrap_err();
        assert!(error.contains("mandatory Quit policy"));
        fs::remove_file(path).ok();
    }

    #[test]
    fn null_key_removal_produces_unbound_not_disabled() {
        let path = unique_path("strata_keymap_unbound_state");
        fs::write(&path, r#"{"keymap":{"f1":null,"?":null}}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(
            keymap.action_state(Action::ToggleKeybindingsHelp),
            ActionBindingState::Unbound
        );
        let f1 = KeyEvent::new(KeyCode::F(1), KeyModifiers::NONE);
        assert_eq!(keymap.resolve_key_event(InputContext::Main, f1), None);
        fs::remove_file(path).ok();
    }

    #[test]
    fn disabled_action_is_not_reached_by_direct_or_contextual_routes() {
        let path = unique_path("strata_keymap_disabled_alias");
        fs::write(&path, r#"{"unbind_actions":["open_layer_popup"]}"#).unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        assert_eq!(keymap.action_state(Action::OpenCategoryModal), ActionBindingState::Disabled);
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let resolved = keymap.resolve_key_event(InputContext::Main, enter).unwrap();
        assert_eq!(resolved.action, Action::Confirm);
        assert_eq!(resolved.source, ResolvedActionSource::Direct);
        fs::remove_file(path).ok();
    }

    #[test]
    fn inherited_aliases_follow_declared_conditions() {
        let keymap = default_keymap();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let today = KeyEvent::new(KeyCode::Char('t'), KeyModifiers::NONE);
        let detach = KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE);

        assert_eq!(
            keymap.resolve_key_event(InputContext::Main, enter).unwrap().action,
            Action::OpenCategoryModal
        );
        assert_eq!(
            keymap.resolve_key_event(InputContext::Main, escape).unwrap().action,
            Action::SwitchToNone
        );
        assert_eq!(
            keymap.resolve_key_event(InputContext::Main, today).unwrap().action,
            Action::ReportToday,
            "detach is directly bound, so target-unbound alias must not apply"
        );
        assert_eq!(
            keymap.resolve_key_event(InputContext::Report, detach).unwrap().action,
            Action::ReportToday
        );
    }

    #[test]
    fn alias_can_be_removed_and_target_can_be_bound() {
        let path = unique_path("strata_keymap_alias_override");
        fs::write(
            &path,
            r#"{
              "keymap":{"ctrl-l":"open_layer_popup"},
              "contextual_aliases":{"main.cancel":null}
            }"#,
        )
        .unwrap();
        let keymap = load_keymap_for_test(&path).unwrap();
        let enter = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let escape = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        assert_eq!(
            keymap.resolve_key_event(InputContext::Main, enter).unwrap().action,
            Action::Confirm,
            "bound target disables target-unbound alias"
        );
        assert_eq!(
            keymap.resolve_key_event(InputContext::Main, escape).unwrap().action,
            Action::Cancel,
            "removed alias leaves source action unchanged"
        );
        fs::remove_file(path).ok();
    }

    #[test]
    fn atlas_unbind_and_disable_persist_distinct_states() {
        let path = unique_path("strata_keymap_atlas_states");
        let unbound = set_action_unbound(&path, Action::OpenReportModal).unwrap();
        assert_eq!(
            unbound.keymap.action_state(Action::OpenReportModal),
            ActionBindingState::Unbound
        );
        let disabled = set_action_binding(&path, Action::OpenReportModal, None).unwrap();
        assert_eq!(
            disabled.keymap.action_state(Action::OpenReportModal),
            ActionBindingState::Disabled
        );
        fs::remove_file(path).ok();
    }
'''
end_marker = "\n}\n"
pos = text.rfind(end_marker)
text = text[:pos] + test_insert + text[pos:]

# Test imports.
text = text.replace(
    '''        Action, KeyBinding, Keymap, default_keymap, load_keybindings, set_action_binding,
        set_first_day_of_week,
''',
    '''        Action, ActionBindingState, InputContext, KeyBinding, Keymap, ResolvedActionSource,
        default_keymap, load_keybindings, set_action_binding, set_action_unbound,
        set_first_day_of_week,
''',
    1,
)
path.write_text(text)

# --- event routing -------------------------------------------------------
path = Path("src/app/event_handlers.rs")
text = path.read_text()
text = text.replace(
    "    keybindings::Action,",
    "    keybindings::{Action, InputContext},",
    1,
)
# Mandatory Ctrl-C resolves before modal-local handlers.
text = text.replace(
    '''        if self.has_persistence_recovery() {
''',
    '''        if self.keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {
            return true;
        }

        if self.has_persistence_recovery() {
''',
    1,
)
# Edit-mode emergency uses mandatory policy only.
text = text.replace(
    "        return if keymap.action_for_key_event(key) == Some(Action::Quit) {",
    "        return if keymap.mandatory_action_for_key_event(key) == Some(Action::Quit) {",
    1,
)
# Remove F1 bypass and resolve explicit context.
old_resolve = '''    fn resolve_action(&self, key: KeyEvent) -> Option<Action> {
        if matches!(key.code, KeyCode::F(1)) {
            return Some(Action::ToggleKeybindingsHelp);
        }

        if self.in_category_modal()
            && !self.show_keybindings_modal
            && matches!(key.code, KeyCode::Char('?'))
        {
            return None;
        }

        self.keymap.action_for_key_event(key)
    }
'''
new_resolve = '''    fn resolve_action(&self, key: KeyEvent) -> Option<Action> {
        if self.in_category_modal()
            && !self.show_keybindings_modal
            && matches!(key.code, KeyCode::Char('?'))
        {
            return None;
        }

        let context = if self.in_karma_modal() {
            InputContext::Report
        } else if self.in_category_modal() || self.show_keybindings_modal {
            InputContext::Other
        } else {
            InputContext::Main
        };
        self.keymap
            .resolve_key_event(context, key)
            .map(|resolved| resolved.action)
    }
'''
if text.count(old_resolve) != 1:
    raise SystemExit("resolve_action block not found")
text = text.replace(old_resolve, new_resolve, 1)
# Palette toggle close must honor direct configured state; mandatory handled above.
text = text.replace(
    "        if self.keymap.action_for_key_event(key) == Some(Action::ToggleCommandPalette) {",
    "        if self\n            .keymap\n            .resolve_key_event(InputContext::Other, key)\n            .is_some_and(|resolved| resolved.action == Action::ToggleCommandPalette)\n        {",
    1,
)
# Atlas capture: Backspace disable, Delete unbind.
old_capture = '''            KeyCode::Backspace | KeyCode::Delete => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_binding(&keymap_path, action, None) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
            }
'''
new_capture = '''            KeyCode::Backspace => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_binding(&keymap_path, action, None) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
            }
            KeyCode::Delete => {
                let keymap_path = crate::storage::get_keymap_path();
                match crate::keybindings::set_action_unbound(&keymap_path, action) {
                    Ok(loaded) => {
                        self.apply_loaded_keybindings(loaded);
                        self.close_atlas_overlay();
                    }
                    Err(err) => {
                        self.keymap_error = Some(err);
                        self.close_atlas_overlay();
                    }
                }
            }
'''
if text.count(old_capture) != 1:
    raise SystemExit("atlas capture delete block not found")
text = text.replace(old_capture, new_capture, 1)
# Hidden contextual handler fallbacks disappear.
text = text.replace(
    '''            Action::Detach => {
                self.set_report_period(ReportPeriod::Today);
            }
''',
    "",
    1,
)
old_main = '''            Action::Confirm => {
                if self
                    .keymap
                    .keys_for_action(Action::OpenCategoryModal)
                    .is_empty()
                {
                    self.open_modal();
                }
                false
            }
'''
if text.count(old_main) != 1:
    raise SystemExit("main confirm fallback not found")
text = text.replace(old_main, "            Action::Confirm => false,\n", 1)
old_cancel = '''            Action::Cancel => {
                if self.keymap.keys_for_action(Action::SwitchToNone).is_empty() {
                    self.queue_or_apply_mutation(QueuedMutation::SwitchLayer(DRIFT_CATEGORY_ID));
                }
                false
            }
'''
if text.count(old_cancel) != 1:
    raise SystemExit("main cancel fallback not found")
text = text.replace(old_cancel, "            Action::Cancel => false,\n", 1)
old_today = '''            Action::ReportToday => {
                if self.keymap.keys_for_action(Action::Detach).is_empty() {
                    self.detach_requested = true;
                    return true;
                }
                self.open_report_modal();
                self.set_report_period(ReportPeriod::Today);
                false
            }
'''
new_today = '''            Action::ReportToday => {
                self.open_report_modal();
                self.set_report_period(ReportPeriod::Today);
                false
            }
'''
if text.count(old_today) != 1:
    raise SystemExit("main today fallback not found")
text = text.replace(old_today, new_today, 1)
path.write_text(text)

# --- app atlas state -----------------------------------------------------
path = Path("src/app.rs")
text = path.read_text()
text = text.replace(
    "    keybindings::{self, Action, ActionCategory, KeyBinding, Keymap},",
    "    keybindings::{self, Action, ActionBindingState, ActionCategory, KeyBinding, Keymap},",
    1,
)
old_effective = '''    pub(super) fn effective_keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        let direct = self.keymap.keys_for_action(action);
        if !direct.is_empty() {
            return direct;
        }

        let fallback_action = match action {
            Action::OpenCategoryModal => Some(Action::Confirm),
            Action::SwitchToNone => Some(Action::Cancel),
            Action::Detach => Some(Action::ReportToday),
            _ => None,
        };

        fallback_action
            .map(|fallback| self.keymap.keys_for_action(fallback))
            .unwrap_or_default()
    }
'''
new_effective = '''    pub(super) fn effective_keys_for_action(&self, action: Action) -> Vec<KeyBinding> {
        let mut keys = self.keymap.keys_for_action(action);
        keys.extend(self.keymap.mandatory_keys_for_action(action));
        keys.sort_by_key(|key| key.to_string());
        keys.dedup();
        keys
    }

    pub(super) fn keymap_state_for_action(&self, action: Action) -> ActionBindingState {
        self.keymap.action_state(action)
    }

    pub(super) fn contextual_labels_for_action(&self, action: Action) -> Vec<String> {
        self.keymap
            .aliases_for_action(action)
            .into_iter()
            .map(|alias| alias.display_label())
            .collect()
    }
'''
if text.count(old_effective) != 1:
    raise SystemExit("effective key fallback block not found")
text = text.replace(old_effective, new_effective, 1)
path.write_text(text)

# --- atlas view ----------------------------------------------------------
path = Path("src/app/keybindings_modal_view.rs")
text = path.read_text()
text = text.replace(
    "    keybindings::{Action, ActionCategory},",
    "    keybindings::{Action, ActionBindingState, ActionCategory},",
    1,
)
# Bottom instructions.
text = text.replace(
    '"Enter edit · Backspace unbind · Esc close",',
    '"Enter edit · Backspace disable · Delete unbind · Esc close",',
    1,
)
# Truthful row labels.
old_row = '''                let keys = self.effective_keys_for_action(action);
                let key_label = if keys.is_empty() {
                    "(unbound)".to_string()
                } else {
                    keys.into_iter()
                        .map(|key| key.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                };
'''
new_row = '''                let direct = self.keymap.keys_for_action(action);
                let mandatory = self.keymap.mandatory_keys_for_action(action);
                let state = self.keymap_state_for_action(action);
                let mut parts = Vec::new();
                match state {
                    ActionBindingState::Bound => parts.push(
                        direct
                            .into_iter()
                            .map(|key| key.to_string())
                            .collect::<Vec<_>>()
                            .join(", "),
                    ),
                    ActionBindingState::Unbound => parts.push("(unbound)".to_string()),
                    ActionBindingState::Disabled => parts.push("(disabled)".to_string()),
                }
                if !mandatory.is_empty() {
                    parts.push(format!(
                        "{} [mandatory]",
                        mandatory
                            .into_iter()
                            .map(|key| key.to_string())
                            .collect::<Vec<_>>()
                            .join(", ")
                    ));
                }
                parts.extend(self.contextual_labels_for_action(action));
                let key_label = parts.join(" · ");
'''
if text.count(old_row) != 1:
    raise SystemExit("atlas row key label block not found")
text = text.replace(old_row, new_row, 1)
# Capture overlay wording.
text = text.replace(
    '"Press a key · Backspace/Delete unbind · Esc cancel",',
    '"Press a key · Backspace disable · Delete unbind · Esc cancel",',
    1,
)
# Atlas tests: defaults now report aliases separately, not synthesized keys.
text = text.replace(
    '''        let open_layer_keys = app.effective_keys_for_action(Action::OpenCategoryModal);
        let switch_idle_keys = app.effective_keys_for_action(Action::SwitchToNone);
        let detach_keys = app.effective_keys_for_action(Action::Detach);

        assert_eq!(open_layer_keys.len(), 1);
        assert_eq!(open_layer_keys[0].to_string(), "Enter");
        assert_eq!(switch_idle_keys.len(), 1);
        assert_eq!(switch_idle_keys[0].to_string(), "Esc");
        assert_eq!(detach_keys.len(), 1);
        assert_eq!(detach_keys[0].to_string(), "d");
''',
    '''        assert_eq!(
            app.keymap_state_for_action(Action::OpenCategoryModal),
            ActionBindingState::Unbound
        );
        assert_eq!(
            app.keymap_state_for_action(Action::SwitchToNone),
            ActionBindingState::Unbound
        );
        assert!(
            app.contextual_labels_for_action(Action::OpenCategoryModal)
                .iter()
                .any(|label| label.contains("main.confirm"))
        );
        assert!(
            app.contextual_labels_for_action(Action::SwitchToNone)
                .iter()
                .any(|label| label.contains("main.cancel"))
        );
        assert_eq!(
            app.keymap_state_for_action(Action::Detach),
            ActionBindingState::Bound
        );
''',
    1,
)
path.write_text(text)

# --- palette -------------------------------------------------------------
path = Path("src/app/command_palette_view.rs")
text = path.read_text()
text = text.replace(
    "    keybindings::Action,",
    "    keybindings::{Action, ActionBindingState},",
    1,
)
# Filter disabled actions after dynamic entries.
old_return = '''        entries
    }

    fn palette_action_entry'''
new_return = '''        entries.retain(|entry| match entry.command {
            PaletteCommand::Action(action) => {
                self.keymap.action_state(action) != ActionBindingState::Disabled
            }
            PaletteCommand::SetReportPeriod(period) => {
                let action = match period {
                    ReportPeriod::Today => Action::ReportToday,
                    ReportPeriod::Week => Action::ReportWeek,
                    ReportPeriod::Month => Action::ReportMonth,
                };
                self.keymap.action_state(action) != ActionBindingState::Disabled
            }
            PaletteCommand::SwitchLayer(_) => true,
        });
        entries
    }

    fn palette_action_entry'''
if text.count(old_return) != 1:
    raise SystemExit("palette entries return anchor not found")
text = text.replace(old_return, new_return, 1)
# Hints expose state rather than aliases-as-bindings.
old_hint = '''    fn palette_hint_for_action(&self, action: Action) -> String {
        let keys = self.effective_keys_for_action(action);
        if keys.is_empty() {
            String::new()
        } else {
            keys.into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        }
    }
'''
new_hint = '''    fn palette_hint_for_action(&self, action: Action) -> String {
        let keys = self.effective_keys_for_action(action);
        if !keys.is_empty() {
            return keys
                .into_iter()
                .map(|key| key.to_string())
                .collect::<Vec<_>>()
                .join(", ");
        }
        match self.keymap.action_state(action) {
            ActionBindingState::Unbound => "unbound".to_string(),
            ActionBindingState::Disabled => "disabled".to_string(),
            ActionBindingState::Bound => String::new(),
        }
    }
'''
if text.count(old_hint) != 1:
    raise SystemExit("palette hint block not found")
text = text.replace(old_hint, new_hint, 1)
path.write_text(text)

for temporary in [
    ".github/workflows/interaction001c-apply.yml",
    "tools/interaction001c-apply.py",
    "tools/interaction001c.trigger",
]:
    Path(temporary).unlink(missing_ok=True)
