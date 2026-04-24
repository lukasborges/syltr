# Syltr para quem vem do Java

Guia rápido pra se orientar no código do Syltr com analogias diretas para o
ecossistema Java/Swing. Assume que você conhece Java 8+ bem e nunca tocou em
C++/Qt — ou tocou há muito tempo.

## TL;DR

- **Qt Widgets em C++ é o C++ mais parecido com Java que existe.** Parent-child
  ownership se comporta quase como GC pra objetos de UI. Signals/slots é o
  padrão observer formalizado. Herança, polimorfismo, encapsulamento — tudo
  familiar.
- **O que é diferente e você tem que absorver:** arquivos `.h` separados do
  `.cpp`, ponteiros crus (mas com regras bem simples), macro `Q_OBJECT` com
  um code generator (MOC), CMake no lugar de Maven/Gradle.
- **O que não muda:** pensar em classes, interfaces, eventos, threads. O
  raciocínio é o mesmo.

## Mapeamento conceitual

| Java / Swing                            | Qt / KF6 (C++)                               |
| --------------------------------------- | -------------------------------------------- |
| `Object`                                | `QObject` (base com meta-objeto)             |
| Garbage Collector                       | Parent-child ownership (`new QFoo(parent)`)  |
| `java.util.List`, `ArrayList`           | `QVector<T>` / `QList<T>`                    |
| `java.util.HashMap`                     | `QHash<K,V>`                                 |
| `String`                                | `QString` (UTF-16, imutável-ish)             |
| Listeners / `addActionListener`         | Signals / slots (`connect(a, &A::sig, b, &B::slot)`) |
| `EventQueue.invokeLater`                | `QMetaObject::invokeMethod` / `QTimer::singleShot(0, ...)` |
| `SwingUtilities.invokeAndWait`          | `BlockingQueuedConnection`                   |
| `JFrame`                                | `QMainWindow`, ou `KXmlGuiWindow` (KDE)      |
| `JPanel`                                | `QWidget`                                    |
| `JList<T>`                              | `QListWidget` / `QListView` + model          |
| `CardLayout`                            | `QStackedWidget`                             |
| `SwingWorker`                           | `QThread` + signals / `QtConcurrent::run`    |
| `ResourceBundle` (i18n)                 | `i18n("texto")` (KLocalizedString)           |
| `SystemTray` / `TrayIcon`               | `KStatusNotifierItem` (KDE nativo)           |
| `final`                                 | `const` (em contextos similares)             |
| Interfaces puras                        | Classes abstratas com métodos virtuais puros |
| `package` / `import`                    | `namespace` / `#include`                     |
| `extends`                               | `: public ParentClass`                       |
| `implements`                            | Herança múltipla (`public Base1, public Base2`) |
| Anotações (`@Override`, `@NotNull`)     | `override`, `const`, `Q_INVOKABLE`, `explicit` |

## Ownership: onde entra "o GC do Qt"

A regra-ouro do Qt: **todo `QObject` recebe um parent**. Quando o parent é
destruído, ele destrói todos os filhos. Você cria objetos com `new`, passa um
pai, e esquece.

```cpp
// Constructor de MainWindow
m_sidebar = new QListWidget(this);   // 'this' é o parent — o MainWindow
m_stack = new QStackedWidget(this);  // ambos morrem quando MainWindow morre
```

É **quase** um GC: difere apenas em que a destruição é determinística (no
momento em que o pai morre), não "quando der vontade".

Quando **não** usar esse padrão:
- Objetos não-`QObject` (dados puros, valor) → use escopo local, `std::unique_ptr`,
  ou copiar por valor (como o struct `Service`).
- Ponteiros que vivem mais que o pai → raro; reestrutura pra evitar.

## Signals e slots = listeners sem boilerplate

```java
// Java / Swing
button.addActionListener(new ActionListener() {
    @Override
    public void actionPerformed(ActionEvent e) {
        doStuff();
    }
});
```

```cpp
// Qt
connect(button, &QPushButton::clicked, this, &MyWindow::doStuff);
```

- Um "signal" é **declarado** na classe (`Q_SIGNALS:`) e **disparado**
  (`Q_EMIT sig();`) — é a interface pública "quando algo acontece".
- Um "slot" é só um método normal; qualquer método compatível serve como slot.
- `connect` aceita lambdas também:
  ```cpp
  connect(button, &QPushButton::clicked, this, [this]() { doStuff(); });
  ```
- Desconexão automática: quando o **receptor é destruído**, o `connect` é
  desfeito. Você **não** tem o problema de listener órfão segurando referência
  que atrapalha GC.

## Headers e implementações: por que dois arquivos?

Em Java tudo vive em `MinhaClasse.java`. Em C++, por tradição (e por causa do
modelo de compilação), a classe é declarada em `.h` (o "contrato") e
implementada em `.cpp` (o "corpo"). Para um programador Java isso é ruído, mas
vale uma analogia: é como se para cada classe Java existissem dois arquivos —
`MinhaClasse.interface.java` com assinaturas, e `MinhaClasse.impl.java` com
bodies.

Na prática no Syltr:

```cpp
// Service.h — o contrato
class Service
{
public:
    Service() = default;
    Service(QString id, QString name, QUrl url, QString iconName);

    QString id() const;   // só assinatura
    // ...
};

// Service.cpp — o corpo
QString Service::id() const { return m_id; }
```

O IDE (KDevelop, Qt Creator, CLion) alterna entre `.h` e `.cpp` com um atalho
(`F12` no KDevelop, `Ctrl+Shift+Up` no Qt Creator).

## O macro `Q_OBJECT` e o MOC

Toda classe que herda de `QObject` e quer signals/slots precisa desta linha:

```cpp
class MainWindow : public KXmlGuiWindow
{
    Q_OBJECT  // <-- obrigatório
    // ...
};
```

O MOC (Meta-Object Compiler) é invocado pelo CMake automaticamente, lê teus
`.h`, encontra `Q_OBJECT`, e gera um `moc_MainWindow.cpp` com o "meta-objeto"
(análogo à reflection do Java). Você nunca edita esse arquivo gerado.

Se você **esquecer** `Q_OBJECT` na classe, vai tomar erro estranho de linker
tipo `undefined reference to vtable for MainWindow`. Quando aparecer, essa é a
causa.

## Sistema de build: CMake ≠ Maven/Gradle, mas resolve o mesmo problema

```cmake
find_package(Qt6 REQUIRED COMPONENTS Core Widgets WebEngineWidgets)
find_package(KF6 REQUIRED COMPONENTS CoreAddons XmlGui Notifications)

add_executable(syltr main.cpp MainWindow.cpp MainWindow.h ...)
target_link_libraries(syltr PRIVATE Qt6::Widgets KF6::XmlGui ...)
```

Analogia mental:
- `find_package(Qt6 COMPONENTS Widgets)` = `<dependency><groupId>qt6</groupId>…`
- `add_executable(syltr ...)` = define o "módulo" principal
- `target_link_libraries` = `<scope>compile</scope>` por dependência

**Fluxo de build:**

```bash
# Equivalente conceitual a `mvn clean install`:
cmake -B build -G Ninja              # gera "project files" (como `mvn generate-sources`)
cmake --build build                  # compila (como `mvn compile`)
./build/bin/syltr                    # roda
```

`build/` é o diretório de saída (tipo `target/` em Maven). Pode apagar a
qualquer momento — é 100% regenerável.

## Tour pelo código do Syltr, em lente Java

### `src/main.cpp`
Ponto de entrada — equivalente ao `public static void main(String[] args)`.

```cpp
int main(int argc, char *argv[]) {
    QApplication app(argc, argv);   // ~ equivalente a iniciar o EDT do Swing
    // ... configura KAboutData, KCrash ...
    auto *window = new MainWindow();
    window->show();
    return app.exec();              // loop de eventos (como SwingUtilities bloqueando)
}
```

### `src/Service.{h,cpp}` — DTO / value type
Equivalente a um `record Service(String id, String name, URL url, String icon)`
do Java 16+. Sem `QObject`, sem parent — é um valor copiável.

### `src/ServiceManager.{h,cpp}` — "loader" + "observable"
Herda `QObject` porque precisa emitir `servicesChanged`. O Java equivalente
seria:

```java
class ServiceManager extends AbstractObservable {
    private List<Service> services = new ArrayList<>();
    void reload() { ... fireServicesChanged(); }
    List<Service> getServices() { return services; }
}
```

### `src/ServiceWebView.{h,cpp}` — extensão de componente
Extende `QWebEngineView` como um `JPanel` que você estende para customizar.
No construtor, cria um `QWebEngineProfile` **com `this` como parent** — então
o profile morre junto com a view. Padrão clássico de "composição com
ownership" Qt.

### `src/MainWindow.{h,cpp}` — o `JFrame` do app
Herda `KXmlGuiWindow` (= `JFrame` com barra de menu gerenciada por XML).
Métodos importantes:

- `setupUi()` — constrói a hierarquia de widgets (equivalente ao que você
  faria em `JFrame.initComponents()`).
- `setupActions()` — registra actions (tipo `KeyStroke`/`Action` do Swing).
- `rebuildServiceViews()` — reage ao sinal `servicesChanged` do manager e
  reconstrói a sidebar + stack. Equivalente a `DefaultListModel.clear() + addElement()`.
- `closeEvent(QCloseEvent *event)` — override de "o usuário clicou no X". Aqui
  a gente chama `event->ignore()` e `hide()` para o app continuar rodando
  minimizado na tray.

### `src/TrayIcon.{h,cpp}` — bandeja do sistema
`KStatusNotifierItem` é o análogo KDE de `java.awt.SystemTray`, mas com
integração Plasma de verdade (ícone no painel, menu com tema nativo).

### `src/syltrui.rc` — layout de menus em XML
Equivalente ao modelo declarativo de menus do Swing via `JMenuBar`, mas
escrito em XML (`kpartgui.dtd`). Permite que o usuário customize menus e
toolbars sem recompilar.

## Armadilhas comuns vindo de Java

1. **`==` em `QString` funciona** (operador sobrecarregado). `==` em ponteiros
   compara endereço, não conteúdo. Então `myString == other` compara texto,
   `myWidget == otherWidget` compara ponteiro. Intuição Java aqui **ajuda**
   em strings, **atrapalha** em objetos.

2. **`const` não é `final`**. `const QString foo()` = "a função não modifica
   `this`". `final QString foo()` em Java seria "a função não pode ser
   sobrescrita". Equivalente de `final` é `final` (existe em C++, mas raro):
   `virtual void foo() final;`.

3. **Passagem por referência vs cópia**. Em Java tudo é por referência
   (exceto primitivos). Em C++, por default é **cópia**. Quando você vê
   `const QString &id` é o equivalente de "passar por referência, somente
   leitura" — é o padrão pra parâmetros não-triviais. Internalize:
   - `QString` sem `&` → cópia (Qt usa copy-on-write, então é barato, mas
     é um hábito ruim).
   - `const QString &` → referência read-only (preferido).
   - `QString &` → referência mutável (raro, usado em out-params).

4. **Ponteiros vs referências**. Em Qt:
   - **Ponteiros (`QWidget *w`)** são usados pra tudo que tem dono (parent)
     e pode ser `nullptr`. Tipo referência nullable do Java.
   - **Referências (`const QString &s`)** são usadas pra parâmetros.
     Não podem ser nulas.

5. **Não há `try/finally`**. Qt evita exceções. Para liberação determinística,
   usa-se **RAII** — scope-based destructor. Exemplo: `QFile f; f.open(...);`
   fecha sozinho ao sair do escopo. Se tivesse que escrever Java equivalente,
   seria um `try-with-resources`.

6. **Não mexa em UI fora da thread principal**. Mesma regra do Swing (EDT).
   Use `QMetaObject::invokeMethod(obj, "slot", Qt::QueuedConnection)` para
   thread-safety em cross-thread signals/slots (o default já cuida disso).

7. **Build lento**. `cmake --build build` reusa o `build/` anterior (como
   `mvn install` sem `clean`). Edits só recompilam o que muda. A primeira
   compilação leva 30-60s, depois é 2-5s.

## Ferramentas recomendadas

- **KDevelop** ou **Qt Creator** como IDE. Ambos entendem CMake, MOC,
  signals/slots, e têm goto-definition decente.
- **CLion** se você já tem licença (interface familiar IntelliJ).
- **`compile_commands.json`** (gerado com `-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`)
  faz qualquer editor com clangd virar um IDE razoável.
- **Qt Assistant** (vem com Qt Creator): documentação offline de toda a Qt
  API, com search-as-you-type. Insubstituível.

## Próximo passo concreto

Com deps instaladas:

```bash
cd ~/IdeaProjects/Syltr
cmake -B build -G Ninja -DCMAKE_BUILD_TYPE=Debug -DCMAKE_EXPORT_COMPILE_COMMANDS=ON
cmake --build build
./build/bin/syltr
```

A janela abre, você vê a sidebar com cinco serviços, e o primeiro (WhatsApp
Web) começa a carregar. Daí já dá pra mexer — abre o `MainWindow.cpp` e tenta
adicionar um botão na sidebar; quando quiser iterar, recompile e roda de novo.
