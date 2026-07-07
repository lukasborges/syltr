# Syltr

Agregador de serviços de mensagens no estilo **Franz**, nativo para o **GNOME**.
Reúne WhatsApp Web, Telegram, Slack, Discord, Messenger e qualquer outro serviço
web numa única janela — cada um com sua **sessão isolada** (cookies e
armazenamento próprios).

**Stack:** GTK4 · libadwaita · WebKitGTK 6 · Rust

## Funcionalidades

- Rail de ícones com favicons reais (rasteriza inclusive SVG) e realce do ativo
- Adicionar serviços a partir de um catálogo ou por URL personalizada
- **Reordenar** serviços arrastando · **menu de contexto** (botão direito)
- **Badges de não lidas** no ícone (detecção por título/DOM)
- **Notificações nativas** do desktop · **silenciar** por serviço · **não perturbe** global
- Sessões isoladas por serviço (login independente em cada um)
- Lista de serviços persistida em `~/.config/dev.syltr.Syltr/services.json`

### Atalhos

| Atalho | Ação |
|--------|------|
| `Ctrl+1` … `Ctrl+9` | Ir para o serviço N |
| `Ctrl+PgDown` / `Alt+↓` | Próximo serviço |
| `Ctrl+PgUp` / `Alt+↑` | Serviço anterior |
| `Ctrl+N` | Adicionar serviço |
| `Ctrl+R` / `F5` | Recarregar |
| `Ctrl+Q` | Sair |

## Desenvolvimento

Instale as dependências (Arch Linux):

```bash
./scripts/install-deps.sh          # tudo, incl. GNOME Builder e CEF (opcional)
./scripts/install-deps.sh --no-cef # sem a engine Chromium (dev com WebKitGTK)
./scripts/install-deps.sh --no-ide # sem GNOME Builder
```

Compile e rode:

```bash
cargo run
```

Ou abra a pasta no **GNOME Builder** (`gnome-builder .`).

## Arquitetura

| Arquivo          | Responsabilidade                                             |
|------------------|-------------------------------------------------------------|
| `src/main.rs`    | Inicializa a `AdwApplication`                                |
| `src/window.rs`  | Janela, sidebar, stack de webviews, ações e diálogos        |
| `src/engine.rs`  | **Camada da engine web** (isolada) — hoje WebKitGTK          |
| `src/config.rs`  | Persistência dos serviços e caminho das sessões (XDG)        |
| `src/catalog.rs` | Catálogo de serviços conhecidos ("recipes")                  |

### Engine web: WebKit hoje, Chromium depois

Todo o app usa **apenas** a API pública de `engine::ServiceView`
(`new`, `widget`, `reload`, `go_home`) e nunca toca no `webkit6` diretamente.
Para migrar para a **engine do Chromium**, basta reimplementar `ServiceView`
em `src/engine.rs` com o crate [`cef`](https://crates.io/crates/cef)
(Chromium Embedded Framework) renderizando *offscreen* dentro de um
`gtk::Widget` — nenhum outro arquivo precisa mudar.

> No Wayland, o CEF exige *offscreen rendering* e a distribuição binária do CEF
> (`CEF_PATH`, veja `scripts/install-deps.sh`). Por isso o desenvolvimento
> começa com WebKitGTK, que é nativo e estável no GNOME.

## Licença

GPL-3.0-or-later
