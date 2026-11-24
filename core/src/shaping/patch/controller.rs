// Standard Uses
use std::borrow::Cow;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;

// Crate Uses
use crate::ast::backend::LanguageBackend;
use crate::ast::listener::ListenerController;
use crate::shaping::lines::{restore, unix};
use crate::shaping::patch::{Actions, Patch};
use crate::shaping::patch::origin::Origin;
use crate::shaping::patch::trace::{ActionTrace, Trace};
use crate::shaping::operation::actions::{Action, ActionOf};
use crate::shaping::operation::actions::builder::BuilderAction;
use crate::shaping::operation::actions::maker::MakerAction;
use crate::shaping::operation::actions::replacer::ReplacerAction;
use crate::shaping::operation::actions::indexer::{ListAction, SwitchAction};

// External Uses
use indextree::{Arena, NodeId};
use miette::Result;


#[allow(unused)]
pub struct PatchController {
    pub patch: Rc<Patch>,
    pub actions: Arena<ActionOf>,
    pub listener_controller: ListenerController,
    pub trace: Option<RefCell<Trace>>,
    paths: HashMap<NodeId, String>,
    origin: Option<Origin>
}

#[allow(unused)]
impl PatchController {
    pub fn with(patch: Rc<Patch>) -> Self {
        Self::with_listener(patch, Default::default())
    }

    pub fn with_listener(patch: Rc<Patch>, listener_controller: ListenerController) -> Self {
        let (actions, paths) = Self::make_actions(patch.clone());

        Self { patch, actions, listener_controller, trace: None, paths, origin: None }
    }

    pub fn traced(patch: Rc<Patch>, limit: usize) -> Self {
        Self { trace: Some(RefCell::new(Trace::new(limit))), ..Self::with(patch) }
    }

    pub fn path(&self, id: NodeId) -> Option<&str> {
        self.paths.get(&id).map(String::as_str)
    }

    pub fn tracing(&self) -> bool {
        self.trace.is_some()
    }

    pub(crate) fn record(&self, id: NodeId, record: impl FnOnce(&mut ActionTrace, usize)) {
        let (Some(trace), Some(path)) = (&self.trace, self.paths.get(&id)) else { return };
        let mut trace = trace.borrow_mut();
        let limit = trace.limit;

        record(trace.actions.entry(path.clone()).or_default(), limit);
    }

    pub(crate) fn place(&self, start: Option<usize>, range: Range<usize>) -> Option<Range<usize>> {
        let start = start?;

        self.origin.as_ref()?.place(start + range.start..start + range.end)
    }

    pub(crate) fn start(&self, location: &str) -> Option<usize> {
        self.listener_controller.location_range.get(location)?.as_ref().map(|range| range.start)
    }

    fn make_actions(patch: Rc<Patch>) -> (Arena<ActionOf>, HashMap<NodeId, String>) {
        let mut arena = Arena::<ActionOf>::new();
        let mut paths = HashMap::new();
        Self::make_inner_actions(&mut arena, &mut paths, &patch.actions, None);
        (arena, paths)
    }

    fn make_inner_actions(
        arena: &mut Arena<ActionOf>, paths: &mut HashMap<NodeId, String>, actions: &Actions, parent: Option<&NodeId>
    ) {
        let parent_path = parent.and_then(|parent| paths.get(parent)).cloned().unwrap_or_default();
        let path = |name: &str| if parent_path.is_empty() { name.to_string() } else { format!("{parent_path}/{name}") };

        if let Some(builders) = &actions.builders {
            for builder in builders.iter() {
                let action = ActionOf::BuilderAction(
                    BuilderAction::with_ref(builder.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().builder_mut().unwrap().post_new(id);
                paths.insert(id, path(&builder.name));

                if let Some(parent) = parent { parent.append(id, arena); }

                Self::make_inner_actions(arena, paths, &builder.actions, Some(&id));
            }
        }

        if let Some(replacers) = &actions.replacers {
            for (index, replacer) in replacers.iter().enumerate() {
                let action = ActionOf::ReplacerAction(
                    ReplacerAction::with_ref(replacer.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().replacer_mut().unwrap().post_new(id);
                paths.insert(id, path(&format!("replacer #{}", index + 1)));

                if let Some(parent) = parent { parent.append(id, arena); }

                Self::make_inner_actions(arena, paths, &replacer.actions, Some(&id));
            }
        }

        if let Some(makers) = &actions.makers {
            for maker in makers.iter() {
                let action = ActionOf::MakerAction(
                    MakerAction::with_ref(maker.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().maker_mut().unwrap().post_new(id);
                paths.insert(id, path(&maker.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(lists) = &actions.lists {
            for list in lists.iter() {
                let action = ActionOf::ListAction(
                    ListAction::with_ref(list.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().list_mut().unwrap().post_new(id);
                paths.insert(id, path(&list.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }

        if let Some(switchers) = &actions.switchers {
            for switch in switchers.iter() {
                let action = ActionOf::SwitchAction(
                    SwitchAction::with_ref(switch.clone(), parent.cloned())
                );
                let id = arena.new_node(action);
                arena.get_mut(id).unwrap().get_mut().switch_mut().unwrap().post_new(id);
                paths.insert(id, path(&switch.name));

                if let Some(parent) = parent { parent.append(id, arena); }
            }
        }
    }

    pub fn apply(&mut self, backend: &dyn LanguageBackend, source: &str) -> Result<String> {
        let unix = unix(source);
        self.origin = self.tracing().then(|| Origin::new(source));
        let result = self.shape(backend, &unix)?;

        Ok(match unix {
            Cow::Borrowed(_) => result,
            Cow::Owned(unix) if unix == result => source.to_string(),
            Cow::Owned(_) => restore(source, &result)
        })
    }

    fn shape(&mut self, backend: &dyn LanguageBackend, source: &str) -> Result<String> {
        let replacer_ids: Vec<NodeId> = self.actions.iter()
            .filter(|node| node.parent().is_none())
            .filter_map(|node| node.get().replacer().ok().map(|replacer| *replacer.self_id()))
            .collect();

        let mut locations = vec![];
        collect_locations(&self.patch.actions, &mut locations);

        let mut source = source.to_string();
        let mut known: Option<HashMap<&str, Vec<Range<usize>>>> = None;

        for id in replacer_ids {
            let replacer = self.actions[id].get().replacer()?.replacer().clone();
            let unlisted = match locations.contains(&replacer.location) {
                true => None,
                false => Some(backend.locate(&source, &replacer.location)?)
            };

            let located = match known.take() {
                Some(located) => located,
                None => {
                    let located = locate(backend, &source, &locations, &replacer.location)?;

                    self.listener_controller.location_contents = located.iter()
                        .map(|(location, ranges)| (
                            location.to_string(),
                            ranges.iter().map(|range| source[range.clone()].to_string()).collect()
                        ))
                        .collect();
                    if self.tracing() {
                        self.listener_controller.location_ranges = located.iter()
                            .map(|(location, ranges)| (location.to_string(), ranges.clone()))
                            .collect();
                        self.listener_controller.location_range = located.keys()
                            .map(|location| (location.to_string(), None))
                            .collect();
                    }

                    located
                }
            };
            let triggers = unlisted.as_ref().or_else(|| located.get(replacer.location.as_str())).into_iter().flatten();

            let mut edits: Vec<(Range<usize>, String)> = vec![];
            for trigger in triggers {
                let mut content = HashMap::new();
                let mut target = None;

                for (location, ranges) in &located {
                    let scoped = scoped_range(ranges, trigger);
                    if let Some(range) = self.listener_controller.location_range.get_mut(*location) { *range = scoped.cloned(); }
                    let Some(range) = scoped else { continue };

                    if *location == replacer.target_location() {
                        target = Some(range.clone());
                    }
                    content.insert(location.to_string(), source[range.clone()].to_string());
                }

                let Some(target) = target else { continue };
                if edits.iter().any(|(range, _)| *range == target) { continue }

                self.listener_controller.location_content = content;
                let Some(replaced) = self.actions[id].get().process(self) else { continue };

                edits.push((target, replaced));
            }

            edits.sort_by_key(|(range, _)| range.start);

            let mut applied: Vec<(Range<usize>, String)> = vec![];
            for edit in edits {
                if applied.last().is_some_and(|(last, _)| edit.0.start < last.end) { continue }
                applied.push(edit);
            }

            if let Some(origin) = &mut self.origin {
                origin.edit(applied.iter().map(|(range, text)| (range.start, &source[range.clone()], text.as_str())));
            }

            let mut changed = false;
            for (range, text) in applied.into_iter().rev() {
                changed |= source[range.clone()] != text;
                source.replace_range(range, &text);
            }

            known = (!changed).then_some(located);
        }

        Ok(source)
    }

    pub fn visit(&mut self) {
        for action in self.actions.iter() {
            let action = action.get();

            match action {
                ActionOf::BuilderAction(b) => {
                    let result = b.process(&self);
                }
                ActionOf::ReplacerAction(r) => {
                    let result = r.process(&self);
                }
                _ => {}
            };
        }
    }

    pub fn find_action<'t>(
        &'t self, name: &str, controller: &'t PatchController
    ) -> Option<&ActionOf> {
        for action in self.actions.iter() {
            let action: &ActionOf = action.get();

            if action.name() == name { return Some(action) }

            if let Some(name) = action.find_inner(name, controller) {
                return Some(name)
            }
        }

        None
    }

    pub fn find_action_mut(&mut self, name: &str) -> Option<&mut ActionOf> {
        for action in self.actions.iter_mut() {
            let action = action.get_mut();

            if action.name() == name { return Some(action) }
        }

        None
    }

    pub fn possible_rules(&self) -> Vec<&'static str> {
        vec!["*"]
    }
}


fn collect_locations(actions: &Actions, locations: &mut Vec<String>) {
    for builder in actions.builders.iter().flatten() {
        locations.push(builder.location.clone());
        locations.extend(builder.reference_location.clone());
        collect_locations(&builder.actions, locations);
    }

    for replacer in actions.replacers.iter().flatten() {
        locations.push(replacer.location.clone());
        locations.push(replacer.reference_location.clone());
        collect_locations(&replacer.actions, locations);
    }

    locations.retain(|location| !location.is_empty());
    locations.sort();
    locations.dedup();
}

fn locate<'l>(
    backend: &dyn LanguageBackend, source: &str, locations: &'l [String], first: &str
) -> Result<HashMap<&'l str, Vec<Range<usize>>>> {
    let wanted: Vec<&str> = locations.iter().map(String::as_str).collect();
    let mut found: Vec<_> = wanted.iter().copied().zip(backend.locate_all(source, &wanted)).collect();

    if let Some(index) = found.iter().position(|(location, ranges)| *location == first && ranges.is_err()) {
        found.swap(0, index);
    }

    found.into_iter().map(|(location, ranges)| Ok((location, ranges?))).collect()
}

fn scoped_range<'r>(ranges: &'r [Range<usize>], trigger: &Range<usize>) -> Option<&'r Range<usize>> {
    ranges.iter()
        .filter(|range| range.start <= trigger.start && trigger.end <= range.end)
        .min_by_key(|range| range.end - range.start)
        .or_else(|| ranges.iter().find(|range| trigger.start <= range.start && range.end <= trigger.end))
}
