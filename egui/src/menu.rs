//! Menu bar functionality (very basic so far).
//!
//! Usage:
//! ```
//! fn show_menu(ui: &mut egui::Ui) {
//!     use egui::{menu, Button};
//!
//!     menu::bar(ui, |ui| {
//!         ui.menu_button("File", |ui| {
//!             if ui.button("Open").clicked() {
//!                 // …
//!             }
//!         });
//!     });
//! }
//! ```

use super::{
    style::{Spacing, WidgetVisuals},
    Align, CtxRef, Id, InnerResponse, PointerState, Pos2, Rect, Response, Sense, Style, TextStyle,
    Ui, Vec2,
};
use crate::{widgets::*, *};
use epaint::{Stroke, mutex::RwLock};
use std::sync::Arc;

/// What is saved between frames.
#[derive(Clone, Default)]
pub(crate) struct BarState {
    open_menu: MenuRootManager,
}

impl BarState {
    fn load(ctx: &Context, bar_id: &Id) -> Self {
        ctx.memory()
            .id_data_temp
            .get_or_default::<Self>(*bar_id)
            .clone()
    }

    fn save(self, ctx: &Context, bar_id: Id) {
        ctx.memory().id_data_temp.insert(bar_id, self);
    }
    /// Show a menu at pointer if right-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn bar_menu<R>(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<InnerResponse<R>> {
        MenuRoot::stationary_click_interaction(response, &mut self.open_menu, response.id);
        self.open_menu.show(response, add_contents)
    }
}
impl std::ops::Deref for BarState {
    type Target = MenuRootManager;
    fn deref(&self) -> &Self::Target {
        &self.open_menu
    }
}
impl std::ops::DerefMut for BarState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.open_menu
    }
}

/// The menu bar goes well in a [`TopBottomPanel::top`],
/// but can also be placed in a `Window`.
/// In the latter case you may want to wrap it in `Frame`.
pub fn bar<R>(ui: &mut Ui, add_contents: impl FnOnce(&mut Ui) -> R) -> InnerResponse<R> {
    ui.horizontal(|ui| {
        let mut style = (**ui.style()).clone();
        style.spacing.button_padding = vec2(2.0, 0.0);
        // style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.active.bg_stroke = Stroke::none();
        // style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.hovered.bg_stroke = Stroke::none();
        style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
        style.visuals.widgets.inactive.bg_stroke = Stroke::none();
        ui.set_style(style);

        // Take full width and fixed height:
        let height = ui.spacing().interact_size.y;
        ui.set_min_size(vec2(ui.available_width(), height));

        add_contents(ui)
    })
}
/// Construct a top level menu in a menu bar. This would be e.g. "File", "Edit" etc.
///
/// Returns `None` if the menu is not open.
pub fn menu_button<R>(
    ui: &mut Ui,
    title: impl ToString,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    stationary_menu_impl(ui, title, Box::new(add_contents))
}
/// Construct a nested sub menu in another menu.
///
/// Returns `None` if the menu is not open.
pub(crate) fn submenu_button<R>(
    ui: &mut Ui,
    parent_state: Arc<RwLock<MenuState>>,
    title: impl ToString,
    add_contents: impl FnOnce(&mut Ui) -> R,
) -> InnerResponse<Option<R>> {
    SubMenu::new(parent_state, title).show(ui, add_contents)
}

/// wrapper for the contents of every menu.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn menu_ui<'c, R>(
    ctx: &CtxRef,
    menu_id: impl std::hash::Hash,
    menu_state_arc: Arc<RwLock<MenuState>>,
    mut style: Style,
    add_contents: impl FnOnce(&mut Ui) -> R + 'c,
) -> InnerResponse<R> {
    let pos = {
        let mut menu_state = menu_state_arc.write();
        menu_state.entry_count = 0;
        menu_state.rect.min
    };
    // style.visuals.widgets.active.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.active.bg_stroke = Stroke::none();
    // style.visuals.widgets.hovered.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.hovered.bg_stroke = Stroke::none();
    style.visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
    style.visuals.widgets.inactive.bg_stroke = Stroke::none();
    let area = Area::new(menu_id)
        .order(Order::Foreground)
        .fixed_pos(pos)
        .interactable(false)
        .drag_bounds(Rect::EVERYTHING);
    let frame = Frame::menu(&style);
    let inner_response = area.show(ctx, |ui| {
        frame
            .show(ui, |ui| {
                const DEFAULT_MENU_WIDTH: f32 = 150.0; // TODO: add to ui.spacing
                ui.set_max_width(DEFAULT_MENU_WIDTH);
                ui.set_style(style);
                ui.set_menu_state(Some(menu_state_arc.clone()));
                ui.with_layout(
                    Layout::top_down_justified(Align::LEFT),
                    add_contents,
                )
                .inner
            })
            .inner
    });
    menu_state_arc.write().rect = inner_response.response.rect;
    inner_response
}

/// build a top level menu with a button
#[allow(clippy::needless_pass_by_value)]
fn stationary_menu_impl<'c, R>(
    ui: &mut Ui,
    title: impl ToString,
    add_contents: Box<dyn FnOnce(&mut Ui) -> R + 'c>,
) -> InnerResponse<Option<R>> {
    let title = title.to_string();
    let bar_id = ui.id();
    let menu_id = bar_id.with(&title);

    let mut bar_state = BarState::load(ui.ctx(), &bar_id);

    let mut button = Button::new(title);

    if bar_state.open_menu.is_menu_open(menu_id) {
        button = button.fill(ui.visuals().widgets.open.bg_fill);
        button = button.stroke(ui.visuals().widgets.open.bg_stroke);
    }

    let button_response = ui.add(button);
    let inner = bar_state.bar_menu(&button_response, add_contents);

    bar_state.save(ui.ctx(), bar_id);
    InnerResponse::new(inner.map(|r| r.inner), button_response)
}

/// Stores the state for the context menu.
#[derive(Default)]
pub(crate) struct ContextMenuSystem {
    root: MenuRootManager,
}
impl ContextMenuSystem {
    /// Show a menu at pointer if right-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn context_menu(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui)) -> Option<InnerResponse<()>> {
        MenuRoot::context_click_interaction(response, &mut self.root, response.id);
        self.root.show(response, add_contents)
    }
}
impl std::ops::Deref for ContextMenuSystem {
    type Target = MenuRootManager;
    fn deref(&self) -> &Self::Target {
        &self.root
    }
}
impl std::ops::DerefMut for ContextMenuSystem {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.root
    }
}

/// Stores the state for the context menu.
#[derive(Clone, Default)]
pub(crate) struct MenuRootManager {
    inner: Option<MenuRoot>,
}
impl MenuRootManager {
    /// Show a menu at pointer if right-clicked response.
    /// Should be called from [`Context`] on a [`Response`]
    pub fn show<R>(&mut self, response: &Response, add_contents: impl FnOnce(&mut Ui) -> R) -> Option<InnerResponse<R>> {
        if let Some(root) = self.inner.as_mut() {
            let (menu_response, inner_response) = root.show(response, add_contents);
            if let MenuResponse::Close = menu_response {
                self.inner = None
            }
            inner_response
        } else {
            None
        }
    }
    fn is_menu_open(&self, id: Id) -> bool {
        self.inner.as_ref().map(|m| m.id) == Some(id)
    }
}
impl std::ops::Deref for MenuRootManager {
    type Target = Option<MenuRoot>;
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
impl std::ops::DerefMut for MenuRootManager {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

/// Menu root associated with an Id from a Response
#[derive(Clone)]
pub(crate) struct MenuRoot {
    pub menu_state: Arc<RwLock<MenuState>>,
    pub id: Id,
}

impl MenuRoot {
    pub fn new(position: Pos2, id: Id) -> Self {
        Self {
            menu_state: Arc::new(RwLock::new(MenuState::new(position))),
            id,
        }
    }
    pub fn show<R>(
        &mut self,
        response: &Response,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> (MenuResponse, Option<InnerResponse<R>>) {
        if self.id == response.id {
            let inner_response = MenuState::show(&response.ctx, &self.menu_state, self.id, add_contents);
            let mut menu_state = self.menu_state.write();
            menu_state.rect = inner_response.response.rect;

            if menu_state.response.is_close() {
                return (MenuResponse::Close, Some(inner_response));
            }
        }
        (MenuResponse::Stay, None)
    }
    /// interaction with a stationary menu, i.e. fixed in another Ui
    fn stationary_interaction(
        response: &Response,
        root: &mut MenuRootManager,
        id: Id,
    ) -> MenuResponse {
        let pointer = &response.ctx.input().pointer;
        if (response.clicked() && root.is_menu_open(id))
            || response.ctx.input().key_pressed(Key::Escape)
        {
            // menu open and button clicked or esc pressed
            return MenuResponse::Close;
        } else if (response.clicked() && !root.is_menu_open(id))
            || (response.hovered() && root.is_some())
        {
            // menu not open and button clicked
            // or button hovered while other menu is open
            let pos = response.rect.left_bottom();
            return MenuResponse::Create(pos, id);
        } else if pointer.any_pressed() && pointer.primary_down() {
            if let Some(pos) = pointer.interact_pos() {
                if let Some(root) = root.inner.as_mut() {
                    if root.id == id {
                        // pressed somewhere while this menu is open
                        let menu_state = root.menu_state.read();
                        let in_menu = menu_state.area_contains(pos);
                        if !in_menu {
                            return MenuResponse::Close;
                        }
                    }
                }
            }
        }
        MenuResponse::Stay
    }
    /// interaction with a context menu
    fn context_interaction(
        response: &Response,
        root: &mut Option<MenuRoot>,
        id: Id,
    ) -> MenuResponse {
        let response = response.interact(Sense::click());
        let pointer = &response.ctx.input().pointer;
        if pointer.any_pressed() {
            if let Some(pos) = pointer.interact_pos() {
                let mut destroy = false;
                let mut in_old_menu = false;
                if let Some(root) = root {
                    let menu_state = root.menu_state.read();
                    in_old_menu = menu_state.area_contains(pos);
                    destroy = root.id == response.id;
                }
                if !in_old_menu {
                    let in_target = response.rect.contains(pos);
                    if in_target && pointer.secondary_down() {
                        return MenuResponse::Create(pos, id);
                    } else if (in_target && pointer.primary_down()) || destroy {
                        return MenuResponse::Close;
                    }
                }
            }
        }
        MenuResponse::Stay
    }
    fn handle_menu_response(
        root: &mut MenuRootManager,
        menu_response: MenuResponse,
    ) {
        match menu_response {
            MenuResponse::Create(pos, id) => {
                root.inner = Some(MenuRoot::new(pos, id));
            }
            MenuResponse::Close => root.inner = None,
            MenuResponse::Stay => {}
        }
    }
    pub fn context_click_interaction(response: &Response, root: &mut MenuRootManager, id: Id) {
        let menu_response = Self::context_interaction(response, root, id);
        Self::handle_menu_response(root, menu_response)
    }
    pub fn stationary_click_interaction(response: &Response, root: &mut MenuRootManager, id: Id) {
        let menu_response = Self::stationary_interaction(response, root, id);
        Self::handle_menu_response(root, menu_response)
    }
}
#[derive(Copy, Clone, PartialEq)]
pub(crate) enum MenuResponse {
    Close,
    Stay,
    Create(Pos2, Id),
}
impl MenuResponse {
    pub fn is_close(&self) -> bool {
        *self == Self::Close
    }
}
pub struct SubMenuButton {
    text: String,
    icon: String,
    index: usize,
}
impl SubMenuButton {
    /// The `icon` can be an emoji (e.g. `⏵` right arrow), shown right of the label
    #[allow(clippy::needless_pass_by_value)]
    fn new(text: impl ToString, icon: impl ToString, index: usize) -> Self {
        Self {
            text: text.to_string(),
            icon: icon.to_string(),
            index,
        }
    }
    fn visuals<'a>(
        ui: &'a Ui,
        response: &'_ Response,
        menu_state: &'_ MenuState,
        sub_id: Id,
    ) -> &'a WidgetVisuals {
        if menu_state.is_open(sub_id) {
            &ui.style().visuals.widgets.hovered
        } else {
            ui.style().interact(response)
        }
    }
    #[allow(clippy::needless_pass_by_value)]
    pub fn icon(mut self, icon: impl ToString) -> Self {
        self.icon = icon.to_string();
        self
    }
    pub(crate) fn show(
        self,
        ui: &mut Ui,
        menu_state: &MenuState,
        sub_id: Id,
    ) -> Response {
        let SubMenuButton {
            text, icon, ..
        } = self;

        let text_style = TextStyle::Button;
        let sense = Sense::click();

        let button_padding = ui.spacing().button_padding;
        let total_extra = button_padding + button_padding;
        let text_available_width = ui.available_width() - total_extra.x;
        let text_galley = ui
            .fonts()
            .layout_delayed_color(text, text_style, text_available_width);

        let icon_available_width = text_available_width - text_galley.size().x;
        let icon_galley = ui
            .fonts()
            .layout_delayed_color(icon, text_style, icon_available_width);
        let text_and_icon_size = Vec2::new(
            text_galley.size().x + icon_galley.size().x,
            text_galley.size().y.max(icon_galley.size().y),
        );
        let desired_size = text_and_icon_size + 2.0 * button_padding;

        let (rect, response) = ui.allocate_at_least(desired_size, sense);
        response.widget_info(|| {
            crate::WidgetInfo::labeled(crate::WidgetType::Button, &text_galley.text())
        });

        if ui.clip_rect().intersects(rect) {
            let visuals = Self::visuals(ui, &response, menu_state, sub_id);
            let text_pos = Align2::LEFT_CENTER
                .align_size_within_rect(text_galley.size(), rect.shrink2(button_padding))
                .min;
            let icon_pos = Align2::RIGHT_CENTER
                .align_size_within_rect(icon_galley.size(), rect.shrink2(button_padding))
                .min;

            ui.painter().rect_filled(
                rect.expand(visuals.expansion),
                visuals.corner_radius,
                visuals.bg_fill,
            );

            let text_color = visuals.text_color();
            ui.painter().galley_with_color(text_pos, text_galley, text_color);
            ui.painter().galley_with_color(icon_pos, icon_galley, text_color);
        }
        response
    }
}
pub struct SubMenu {
    button: SubMenuButton,
    parent_state: Arc<RwLock<MenuState>>,
}
impl SubMenu {
    #[allow(clippy::needless_pass_by_value)]
    fn new(parent_state: Arc<RwLock<MenuState>>, text: impl ToString) -> Self {
        let index = parent_state.write().next_entry_index();
        Self {
            button: SubMenuButton::new(text, "⏵", index),
            parent_state,
        }
    }
    pub fn show<R>(
        self,
        ui: &mut Ui,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<Option<R>> {
        let sub_id = ui.id().with(self.button.index);
        let button = self.button.show(
            ui,
            &*self.parent_state.read(),
            sub_id,
        );
        self.parent_state
            .write()
            .submenu_button_interaction(ui, sub_id, &button);
        let inner = self
            .parent_state
            .write()
            .show_submenu(ui.ctx(), sub_id, add_contents);
        InnerResponse::new(inner, button)
    }
}
pub(crate) struct MenuState {
    /// The opened sub-menu and its `Id`
    sub_menu: Option<(Id, Arc<RwLock<MenuState>>)>,
    /// Bounding box of this menu (without the sub-menu)
    pub rect: Rect,
    /// Used to check if any menu in the tree wants to close
    pub response: MenuResponse,
    /// Used to hash different `Id`s for sub-menus
    entry_count: usize,
}
impl MenuState {
    pub fn new(position: Pos2) -> Self {
        Self {
            rect: Rect::from_min_size(position, Vec2::ZERO),
            sub_menu: None,
            response: MenuResponse::Stay,
            entry_count: 0,
        }
    }
    /// Close menu hierarchy.
    pub fn close(&mut self) {
        self.response = MenuResponse::Close;
    }
    pub fn show<R>(
        ctx: &CtxRef,
        menu_state: &Arc<RwLock<Self>>,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> InnerResponse<R> {
        let style = Style {
            spacing: Spacing {
                item_spacing: Vec2::ZERO,
                button_padding: crate::vec2(2.0, 0.0),
                ..Default::default()
            },
            ..Default::default()
        };
        let menu_state_arc = menu_state.clone();
        crate::menu::menu_ui(ctx, id, menu_state_arc, style, add_contents)
    }
    fn show_submenu<R>(
        &mut self,
        ctx: &CtxRef,
        id: Id,
        add_contents: impl FnOnce(&mut Ui) -> R,
    ) -> Option<R> {
        let (sub_response, response) = self.get_submenu(id).map(|sub| {
            let inner_response = Self::show(ctx, sub, id, add_contents);
            (sub.read().response, inner_response.inner)
        })?;
        self.cascade_close_response(sub_response);
        Some(response)
    }
    /// Check if position is in the menu hierarchy's area.
    pub fn area_contains(&self, pos: Pos2) -> bool {
        self.rect.contains(pos)
            || self
                .sub_menu
                .as_ref()
                .map(|(_, sub)| sub.read().area_contains(pos))
                .unwrap_or(false)
    }
    fn next_entry_index(&mut self) -> usize {
        self.entry_count += 1;
        self.entry_count - 1
    }
    /// Sense button interaction opening and closing submenu.
    fn submenu_button_interaction(&mut self, ui: &mut Ui, sub_id: Id, button: &Response) {
        let pointer = &ui.input().pointer.clone();
        let open = self.is_open(sub_id);
        if self.moving_towards_current_submenu(pointer) {
            // ensure to repaint once even when pointer is not moving
            ui.ctx().request_repaint();
        } else if !open && button.hovered() {
            let pos = button.rect.right_top();
            self.open_submenu(sub_id, pos);
        } else if open && !button.hovered() && !self.hovering_current_submenu(pointer) {
            self.close_submenu();
        }
    }
    /// Check if `dir` points from `pos` towards left side of `rect`.
    fn points_at_left_of_rect(pos: Pos2, dir: Vec2, rect: Rect) -> bool {
        let vel_a = dir.angle();
        let top_a = (rect.left_top() - pos).angle();
        let bottom_a = (rect.left_bottom() - pos).angle();
        bottom_a - vel_a >= 0.0 && top_a - vel_a <= 0.0
    }
    /// Check if pointer is moving towards current submenu.
    fn moving_towards_current_submenu(&self, pointer: &PointerState) -> bool {
        if pointer.is_still() {
            return false;
        }
        if let Some(sub_menu) = self.get_current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return Self::points_at_left_of_rect(
                    pos,
                    pointer.velocity(),
                    sub_menu.read().rect,
                );
            }
        }
        false
    }
    /// Check if pointer is hovering current submenu.
    fn hovering_current_submenu(&self, pointer: &PointerState) -> bool {
        if let Some(sub_menu) = self.get_current_submenu() {
            if let Some(pos) = pointer.hover_pos() {
                return sub_menu.read().area_contains(pos);
            }
        }
        false
    }
    /// Cascade close response to menu root.
    fn cascade_close_response(&mut self, response: MenuResponse) {
        if response.is_close() {
            self.response = response;
        }
    }
    fn is_open(&self, id: Id) -> bool {
        self.get_sub_id() == Some(id)
    }
    fn get_sub_id(&self) -> Option<Id> {
        self.sub_menu.as_ref().map(|(id, _)| *id)
    }
    fn get_current_submenu(&self) -> Option<&Arc<RwLock<MenuState>>> {
        self.sub_menu.as_ref().map(|(_, sub)| sub)
    }
    fn get_submenu(&mut self, id: Id) -> Option<&Arc<RwLock<MenuState>>> {
        self.sub_menu
            .as_ref()
            .and_then(|(k, sub)| if id == *k { Some(sub) } else { None })
    }
    /// Open submenu at position, if not already open.
    fn open_submenu(&mut self, id: Id, pos: Pos2) {
        if !self.is_open(id) {
            self.sub_menu = Some((id, Arc::new(RwLock::new(MenuState::new(pos)))));
        }
    }
    fn close_submenu(&mut self) {
        self.sub_menu = None;
    }
}
