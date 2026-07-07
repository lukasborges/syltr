//! Janela principal — layout GNOME com AdwNavigationSplitView:
//! sidebar de serviços à esquerda, stack de webviews à direita.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use adw::prelude::*;
use gtk::{gdk, gio, glib};

use crate::config::{self, Service};
use crate::{catalog, engine};

const EMPTY_PAGE: &str = "__empty__";

/// Estado mutável compartilhado entre os callbacks.
struct State {
    services: Vec<Service>,
    views: HashMap<String, engine::ServiceView>,
    current: Option<String>,
}

/// Handle clonável para a janela e seus widgets principais.
#[derive(Clone)]
struct Ui {
    window: adw::ApplicationWindow,
    split: adw::NavigationSplitView,
    list: gtk::ListBox,
    stack: gtk::Stack,
    title: adw::WindowTitle,
    app: adw::Application,
    /// modo "não perturbe" global (suprime todas as notificações)
    dnd: Rc<Cell<bool>>,
    state: Rc<RefCell<State>>,
}

/// Ponto de entrada: constrói (ou reapresenta) a janela do app.
pub fn build(app: &adw::Application) {
    if let Some(win) = app.active_window() {
        win.present();
        return;
    }

    let services = config::load();
    let state = Rc::new(RefCell::new(State {
        services,
        views: HashMap::new(),
        current: None,
    }));

    // ---- Rail lateral (só ícones), sem header próprio --------------------
    let list = gtk::ListBox::builder()
        .selection_mode(gtk::SelectionMode::Single)
        .css_classes(["navigation-sidebar", "rail"])
        .build();

    let sidebar_scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&list)
        .vexpand(true)
        .build();

    let sidebar_page = adw::NavigationPage::builder()
        .title("Serviços")
        .child(&sidebar_scroll)
        .build();

    // ---- Conteúdo (stack de webviews), sem header próprio ----------------
    let stack = gtk::Stack::builder()
        .transition_type(gtk::StackTransitionType::Crossfade)
        .vexpand(true)
        .hexpand(true)
        .build();
    stack.add_named(&empty_state(), Some(EMPTY_PAGE));

    let content_page = adw::NavigationPage::builder()
        .title("Syltr")
        .child(&stack)
        .build();

    // Rail estreito + conteúdo, LOGO ABAIXO do header único.
    let split = adw::NavigationSplitView::builder()
        .sidebar(&sidebar_page)
        .content(&content_page)
        .min_sidebar_width(84.0)
        .max_sidebar_width(84.0)
        .build();

    // ---- Header ÚNICO, cobrindo toda a largura da janela -----------------
    let menu_button = gtk::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Menu principal")
        .menu_model(&primary_menu())
        .primary(true)
        .build();
    let reload_button = gtk::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Recarregar")
        .action_name("win.reload")
        .build();
    let home_button = gtk::Button::builder()
        .icon_name("go-home-symbolic")
        .tooltip_text("Início do serviço")
        .action_name("win.home")
        .build();

    let title = adw::WindowTitle::new("Syltr", "");

    let top_header = adw::HeaderBar::new();
    top_header.pack_start(&menu_button);
    top_header.pack_start(&reload_button);
    top_header.pack_start(&home_button);
    top_header.set_title_widget(Some(&title));

    let root_toolbar = adw::ToolbarView::new();
    root_toolbar.add_top_bar(&top_header);
    root_toolbar.set_content(Some(&split));

    let window = adw::ApplicationWindow::builder()
        .application(app)
        .title("Syltr")
        .default_width(1100)
        .default_height(760)
        .width_request(360)
        .height_request(360)
        .content(&root_toolbar)
        .build();

    let ui = Ui {
        window: window.clone(),
        split,
        list: list.clone(),
        stack,
        title,
        app: app.clone(),
        dnd: Rc::new(Cell::new(false)),
        state,
    };

    wire_actions(app, &ui);

    // Seleciona linha -> mostra serviço.
    {
        let ui = ui.clone();
        list.connect_row_selected(move |_, row| {
            if let Some(row) = row {
                ui.show_service_at(row.index() as usize);
            }
        });
    }

    ui.refresh_sidebar();
    window.present();
}

impl Ui {
    /// Garante que a webview de um serviço exista (carrega em segundo plano) e
    /// devolve um clone dela. Chamado no início para todos os serviços, para
    /// que os favicons carreguem e as notificações cheguem como no Franz.
    fn ensure_view(&self, svc: &Service) -> engine::ServiceView {
        if let Some(view) = self.state.borrow().views.get(&svc.id) {
            return view.clone();
        }
        let view = engine::ServiceView::new(
            &svc.id,
            &svc.name,
            &svc.url,
            &config::session_dir(&svc.id),
            &self.app,
            self.dnd.clone(),
            svc.muted,
        );
        self.stack.add_named(view.widget(), Some(&svc.id));
        self.state
            .borrow_mut()
            .views
            .insert(svc.id.clone(), view.clone());
        view
    }

    /// Reconstrói o rail lateral a partir do estado atual.
    fn refresh_sidebar(&self) {
        while let Some(child) = self.list.first_child() {
            self.list.remove(&child);
        }
        let services = self.state.borrow().services.clone();
        for (i, svc) in services.iter().enumerate() {
            let view = self.ensure_view(svc);
            let row = service_row(svc, view.icon());
            self.attach_row_controllers(&row, i);
            self.list.append(&row);
        }
        if services.is_empty() {
            self.stack.set_visible_child_name(EMPTY_PAGE);
            self.title.set_title("Syltr");
            self.state.borrow_mut().current = None;
        } else {
            // Mantém seleção válida (ou seleciona o primeiro).
            let idx = self
                .state
                .borrow()
                .current
                .as_ref()
                .and_then(|cur| services.iter().position(|s| &s.id == cur))
                .unwrap_or(0);
            self.select_index(idx);
        }
    }

    fn select_index(&self, idx: usize) {
        if let Some(row) = self.list.row_at_index(idx as i32) {
            self.list.select_row(Some(&row));
        }
    }

    /// Anexa a uma célula do rail: arrastar-para-reordenar e menu de contexto.
    fn attach_row_controllers(&self, row: &gtk::ListBoxRow, index: usize) {
        // Arrastar (fonte): carrega o índice de origem.
        let drag = gtk::DragSource::new();
        drag.set_actions(gdk::DragAction::MOVE);
        let from = index as i32;
        drag.connect_prepare(move |_, _, _| {
            Some(gdk::ContentProvider::for_value(&from.to_value()))
        });
        row.add_controller(drag);

        // Soltar (alvo): move o serviço de origem para esta posição.
        let drop = gtk::DropTarget::new(glib::Type::I32, gdk::DragAction::MOVE);
        let ui = self.clone();
        let to = index;
        drop.connect_drop(move |_, value, _, _| {
            if let Ok(src) = value.get::<i32>() {
                ui.move_service(src as usize, to);
                return true;
            }
            false
        });
        row.add_controller(drop);

        // Botão direito: menu de contexto do serviço.
        let click = gtk::GestureClick::new();
        click.set_button(gdk::BUTTON_SECONDARY);
        let ui = self.clone();
        let row_weak = row.downgrade();
        click.connect_pressed(move |_, _, x, y| {
            if let Some(row) = row_weak.upgrade() {
                ui.show_context_menu(index, &row, x, y);
            }
        });
        row.add_controller(click);
    }

    /// Reordena o serviço `from` para a posição `to`, persiste e reconstrói.
    fn move_service(&self, from: usize, to: usize) {
        if from == to {
            return;
        }
        {
            let mut st = self.state.borrow_mut();
            if from >= st.services.len() || to >= st.services.len() {
                return;
            }
            let svc = st.services.remove(from);
            st.services.insert(to, svc);
        }
        self.save();
        self.refresh_sidebar();
    }

    /// Passo relativo na lista de serviços (para atalhos próximo/anterior).
    fn step(&self, delta: i32) {
        let (len, cur) = {
            let st = self.state.borrow();
            let len = st.services.len() as i32;
            let cur = st
                .current
                .as_ref()
                .and_then(|id| st.services.iter().position(|s| &s.id == id))
                .map(|p| p as i32)
                .unwrap_or(0);
            (len, cur)
        };
        if len == 0 {
            return;
        }
        let next = (((cur + delta) % len) + len) % len;
        self.select_index(next as usize);
    }

    /// Mostra o menu de contexto do serviço `index` ancorado em `row`.
    fn show_context_menu(&self, index: usize, row: &gtk::ListBoxRow, x: f64, y: f64) {
        // Seleciona o serviço clicado (as ações operam sobre o atual).
        self.select_index(index);

        // Sincroniza o estado do item "Silenciar" com o serviço atual.
        let muted = self
            .state
            .borrow()
            .services
            .get(index)
            .map(|s| s.muted)
            .unwrap_or(false);
        if let Some(action) = self.window.lookup_action("mute") {
            if let Ok(simple) = action.downcast::<gio::SimpleAction>() {
                simple.set_state(&muted.to_variant());
            }
        }

        let menu = gio::Menu::new();
        let s1 = gio::Menu::new();
        s1.append(Some("Recarregar"), Some("win.reload"));
        s1.append(Some("Início do serviço"), Some("win.home"));
        menu.append_section(None, &s1);
        let s2 = gio::Menu::new();
        s2.append(Some("Silenciar notificações"), Some("win.mute"));
        menu.append_section(None, &s2);
        let s3 = gio::Menu::new();
        s3.append(Some("Remover serviço"), Some("win.remove-service"));
        menu.append_section(None, &s3);

        let popover = gtk::PopoverMenu::from_model(Some(&menu));
        popover.set_parent(row);
        popover.set_has_arrow(false);
        popover.set_pointing_to(Some(&gdk::Rectangle::new(x as i32, y as i32, 1, 1)));
        popover.connect_closed(|p| p.unparent());
        popover.popup();
    }

    /// Aplica silenciar/dessilenciar ao serviço atual (persiste).
    fn set_current_muted(&self, muted: bool) {
        let cur = self.state.borrow().current.clone();
        if let Some(id) = cur {
            {
                let mut st = self.state.borrow_mut();
                if let Some(svc) = st.services.iter_mut().find(|s| s.id == id) {
                    svc.muted = muted;
                }
                if let Some(view) = st.views.get(&id) {
                    view.set_muted(muted);
                }
            }
            self.save();
        }
    }

    /// Garante que a webview do serviço `idx` exista e a exibe.
    fn show_service_at(&self, idx: usize) {
        let (id, name) = {
            let st = self.state.borrow();
            match st.services.get(idx) {
                Some(s) => (s.id.clone(), s.name.clone()),
                None => return,
            }
        };

        self.stack.set_visible_child_name(&id);
        self.title.set_title(&name);
        self.state.borrow_mut().current = Some(id);
        self.split.set_show_content(true);
    }

    fn add_service(&self, name: &str, url: &str) {
        let id = {
            let st = self.state.borrow();
            config::make_id(&st.services, name)
        };
        self.state.borrow_mut().services.push(Service {
            id: id.clone(),
            name: name.to_string(),
            url: config::normalize_url(url),
            muted: false,
        });
        self.save();
        self.state.borrow_mut().current = Some(id);
        self.refresh_sidebar();
    }

    fn remove_current(&self) {
        let Some(id) = self.state.borrow().current.clone() else {
            return;
        };
        {
            let mut st = self.state.borrow_mut();
            st.services.retain(|s| s.id != id);
            if let Some(view) = st.views.remove(&id) {
                self.stack.remove(view.widget());
            }
            st.current = None;
        }
        self.save();
        self.refresh_sidebar();
    }

    fn current_view(&self) -> Option<engine::ServiceView> {
        let st = self.state.borrow();
        st.current.as_ref().and_then(|id| st.views.get(id).cloned())
    }

    fn save(&self) {
        config::save(&self.state.borrow().services);
    }
}

/// Constrói uma célula do rail: só o ícone (favicon/inicial) do serviço,
/// centralizado, com o nome no tooltip ao passar o mouse.
fn service_row(svc: &Service, icon: &gtk::Widget) -> gtk::ListBoxRow {
    let bx = gtk::Box::builder()
        .orientation(gtk::Orientation::Horizontal)
        .halign(gtk::Align::Center)
        .margin_top(6)
        .margin_bottom(6)
        .build();
    bx.append(icon);

    gtk::ListBoxRow::builder()
        .child(&bx)
        .tooltip_text(&svc.name)
        .build()
}

/// Página mostrada quando não há nenhum serviço.
fn empty_state() -> adw::StatusPage {
    let button = gtk::Button::builder()
        .label("Adicionar serviço")
        .halign(gtk::Align::Center)
        .action_name("win.add-service")
        .css_classes(["suggested-action", "pill"])
        .build();

    adw::StatusPage::builder()
        .icon_name("chat-symbolic")
        .title("Nenhum serviço")
        .description("Adicione um serviço de mensagens para começar.")
        .child(&button)
        .build()
}

fn primary_menu() -> gio::Menu {
    let menu = gio::Menu::new();

    let section = gio::Menu::new();
    section.append(Some("Adicionar serviço"), Some("win.add-service"));
    section.append(Some("Remover serviço atual"), Some("win.remove-service"));
    menu.append_section(None, &section);

    let dnd = gio::Menu::new();
    dnd.append(Some("Não perturbe"), Some("win.toggle-dnd"));
    menu.append_section(None, &dnd);

    let about = gio::Menu::new();
    about.append(Some("Sobre o Syltr"), Some("app.about"));
    about.append(Some("Sair"), Some("app.quit"));
    menu.append_section(None, &about);

    menu
}

fn wire_actions(app: &adw::Application, ui: &Ui) {
    // win.add-service
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("add-service", None);
        action.connect_activate(move |_, _| show_add_dialog(&uic));
        ui.window.add_action(&action);
    }
    // win.reload
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("reload", None);
        action.connect_activate(move |_, _| {
            if let Some(view) = uic.current_view() {
                view.reload();
            }
        });
        ui.window.add_action(&action);
    }
    // win.home
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("home", None);
        action.connect_activate(move |_, _| {
            if let Some(view) = uic.current_view() {
                view.go_home();
            }
        });
        ui.window.add_action(&action);
    }
    // win.remove-service
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("remove-service", None);
        action.connect_activate(move |_, _| uic.remove_current());
        ui.window.add_action(&action);
    }
    // win.mute (toggle silenciar do serviço atual)
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new_stateful("mute", None, &false.to_variant());
        action.connect_change_state(move |a, value| {
            if let Some(v) = value {
                let m: bool = v.get().unwrap_or(false);
                uic.set_current_muted(m);
                a.set_state(v);
            }
        });
        ui.window.add_action(&action);
    }
    // win.toggle-dnd (não perturbe global)
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new_stateful("toggle-dnd", None, &false.to_variant());
        action.connect_change_state(move |a, value| {
            if let Some(v) = value {
                uic.dnd.set(v.get().unwrap_or(false));
                a.set_state(v);
            }
        });
        ui.window.add_action(&action);
    }
    // win.goto1..9 (Ctrl+1..9) e próximo/anterior
    for i in 1usize..=9 {
        let uic = ui.clone();
        let action = gio::SimpleAction::new(&format!("goto{i}"), None);
        action.connect_activate(move |_, _| uic.select_index(i - 1));
        ui.window.add_action(&action);
        app.set_accels_for_action(&format!("win.goto{i}"), &[&format!("<Primary>{i}")]);
    }
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("next-service", None);
        action.connect_activate(move |_, _| uic.step(1));
        ui.window.add_action(&action);
        app.set_accels_for_action("win.next-service", &["<Primary>Page_Down", "<Alt>Down"]);
    }
    {
        let uic = ui.clone();
        let action = gio::SimpleAction::new("prev-service", None);
        action.connect_activate(move |_, _| uic.step(-1));
        ui.window.add_action(&action);
        app.set_accels_for_action("win.prev-service", &["<Primary>Page_Up", "<Alt>Up"]);
    }
    // app.about
    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("about", None);
        action.connect_activate(move |_, _| show_about(&win));
        app.add_action(&action);
    }
    // app.quit
    {
        let win = ui.window.clone();
        let action = gio::SimpleAction::new("quit", None);
        action.connect_activate(move |_, _| win.close());
        app.add_action(&action);
        app.set_accels_for_action("app.quit", &["<Primary>q"]);
    }
    app.set_accels_for_action("win.reload", &["<Primary>r", "F5"]);
    app.set_accels_for_action("win.add-service", &["<Primary>n"]);
}

/// Diálogo "Adicionar serviço": catálogo + URL personalizada.
fn show_add_dialog(ui: &Ui) {
    let dialog = adw::Dialog::builder()
        .title("Adicionar serviço")
        .content_width(460)
        .content_height(600)
        .build();

    // Catálogo
    let catalog_group = adw::PreferencesGroup::builder().title("Serviços").build();
    for entry in catalog::CATALOG {
        let row = adw::ActionRow::builder()
            .title(entry.name)
            .subtitle(entry.url)
            .activatable(true)
            .build();
        row.add_prefix(&adw::Avatar::new(28, Some(entry.name), true));
        row.add_suffix(&gtk::Image::from_icon_name("list-add-symbolic"));

        let ui = ui.clone();
        let dialog = dialog.clone();
        let name = entry.name.to_string();
        let url = entry.url.to_string();
        row.connect_activated(move |_| {
            ui.add_service(&name, &url);
            dialog.close();
        });
        catalog_group.add(&row);
    }

    // Personalizado
    let custom_group = adw::PreferencesGroup::builder()
        .title("Personalizado")
        .description("Adicione qualquer serviço web pela URL.")
        .build();

    let name_row = adw::EntryRow::builder().title("Nome").build();
    let url_row = adw::EntryRow::builder().title("URL (https://…)").build();
    custom_group.add(&name_row);
    custom_group.add(&url_row);

    let add_custom = gtk::Button::builder()
        .label("Adicionar")
        .halign(gtk::Align::End)
        .margin_top(12)
        .css_classes(["suggested-action"])
        .build();
    {
        let ui = ui.clone();
        let dialog = dialog.clone();
        let name_row = name_row.clone();
        let url_row = url_row.clone();
        add_custom.connect_clicked(move |_| {
            let url = url_row.text().to_string();
            if url.trim().is_empty() {
                url_row.add_css_class("error");
                return;
            }
            let mut name = name_row.text().to_string();
            if name.trim().is_empty() {
                name = "Serviço".to_string();
            }
            ui.add_service(&name, &url);
            dialog.close();
        });
    }

    let content = gtk::Box::builder()
        .orientation(gtk::Orientation::Vertical)
        .spacing(18)
        .margin_top(12)
        .margin_bottom(18)
        .margin_start(18)
        .margin_end(18)
        .build();
    content.append(&catalog_group);
    content.append(&custom_group);
    content.append(&add_custom);

    let scroll = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&content)
        .vexpand(true)
        .build();

    let header = adw::HeaderBar::new();
    let toolbar = adw::ToolbarView::new();
    toolbar.add_top_bar(&header);
    toolbar.set_content(Some(&scroll));

    dialog.set_child(Some(&toolbar));
    dialog.present(Some(&ui.window));
}

fn show_about(parent: &impl IsA<gtk::Widget>) {
    let about = adw::AboutDialog::builder()
        .application_name("Syltr")
        .application_icon(crate::APP_ID)
        .developer_name("Lucas Borges")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("Agregador de serviços de mensagens estilo Franz para o GNOME.")
        .website("https://github.com/")
        .license_type(gtk::License::Gpl30)
        .build();
    about.present(Some(parent));
}
