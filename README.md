<h1 align="center">PdfCraft RU</h1>
<p align="center"><b>Работа с PDF на базе открытого проекта PdfCraft</b><br>Просмотр · Страницы · Аннотации · Формы · CLI</p>
<p align="center"><a href="https://github.com/maxliv234-star/pdfcraft/tree/feature/ru-edition">Ветка RU-редакции</a> · <a href="docs/localization.md">Языки интерфейса</a> · <a href="docs/development.md">Разработка</a> · <a href="https://github.com/storytold/pdfcraft">Исходный проект</a></p>

---

**Этот репозиторий — форк [storytold/pdfcraft](https://github.com/storytold/pdfcraft)**, проекта ArtCraft Team и сообщества. Здесь ведётся работа над RU-редакцией. PDF-движок, основной интерфейс и большая часть возможностей созданы авторами исходного проекта.

> **Статус: в разработке.** Ветка `main` содержит базовую версию PdfCraft; работа над RU-редакцией ведётся в `feature/ru-edition`. Отдельный стабильный релиз этого форка пока не опубликован. Наличие русского языка в базовом PdfCraft само по себе не является отличием форка.

## Что умеет базовый PdfCraft

- Просмотр PDF, поиск и извлечение текста.
- Поворот, удаление, перестановка и извлечение страниц.
- Объединение и разделение документов.
- Аннотации, комментарии и заполнение форм.
- Работа с закладками, метаданными и защищёнными документами.
- Командная строка и включаемый отдельно MCP-сервер для автоматизации.

Реализация и ограничения отдельных функций описаны в [ROADMAP.md](ROADMAP.md). Редактирование существующего текста, OCR и сложные профессиональные PDF-сценарии продолжают развиваться.

## RU-редакция

| Ветка | Назначение |
| :--- | :--- |
| [`main`](https://github.com/maxliv234-star/pdfcraft/tree/main) | База исходного проекта и документация форка |
| [`feature/ru-edition`](https://github.com/maxliv234-star/pdfcraft/tree/feature/ru-edition) | Работа над RU-редакцией |
| [`feature/ru-ui-polish-and-releases`](https://github.com/maxliv234-star/pdfcraft/tree/feature/ru-ui-polish-and-releases) | Изменения типографики, компактных карточек и подготовки Windows-релизов |

Изменения интерфейса и выпуск сборок рассматриваются в [PR #1](https://github.com/maxliv234-star/pdfcraft/pull/1). Наличие ветки или pull request не означает, что изменения уже входят в `main` или выпущенный релиз.

## Запуск из исходников

Требуются Rust **1.90+**, Cargo и нативные инструменты сборки для вашей ОС. На Windows используется набор MSVC Build Tools.

```bash
git clone https://github.com/maxliv234-star/pdfcraft.git
cd pdfcraft
cargo run --release -p pdfcraft -- example.pdf
```

Для проверки RU-ветки после клонирования:

```bash
git switch feature/ru-edition
cargo run --release -p pdfcraft -- example.pdf
```

Русский язык выбирается в **Menu → Edit → Preferences → Interface language → Русский**. Режим Auto следует языку системы. Подробности и ограничения покрытия — в [документации локализации](docs/localization.md).

Сборка из исходников не равнозначна подписанному установщику. Доступные официальные сборки **исходного PdfCraft** находятся в [Releases storytold/pdfcraft](https://github.com/storytold/pdfcraft/releases); они не являются сборками RU-форка.

## Командная строка

```bash
cargo run --release -p pdfcraft-cli -- info example.pdf
cargo run --release -p pdfcraft-cli -- text example.pdf --page 1
cargo run --release -p pdfcraft-cli -- tools
```

CLI, сценарии и MCP используют общий слой автоматизации. MCP включается только явным запуском; руководство — в [crates/automation/README.md](crates/automation/README.md).

## Структура

| Путь | Назначение |
| :--- | :--- |
| `apps/pdfcraft/` | Desktop-приложение |
| `apps/pdfcraft-cli/` | Командная строка и автоматизация |
| `apps/pdfcraft-web/` | WebAssembly-интерфейс |
| `crates/` | PDF-движок и функциональные модули |
| `crates/ui-egui/` | Интерфейс на egui и каталоги переводов |
| `xtask/` | Проверки, сборка ресурсов и инструменты разработки |
| `docs/` | Разработка, локализация и релизы |
| `packaging/` | Упаковка под разные платформы |

## Проверки и участие

```bash
cargo fmt --check
cargo clippy --workspace -- -D warnings
cargo test --workspace
```

Полный набор проверок описан в [CLAUDE.md](CLAUDE.md); правила проекта — в [AGENTS.md](AGENTS.md).
Для ошибок используйте [Issues](https://github.com/maxliv234-star/pdfcraft/issues), для предложений по коду — [Pull requests](https://github.com/maxliv234-star/pdfcraft/pulls). При воспроизведении ошибок используйте искусственные или обезличенные PDF.

## Лицензия и авторство

Основа PdfCraft распространяется по [MIT](LICENSE-MIT) или [Apache-2.0](LICENSE-APACHE), на ваш выбор.
© 2026 ArtCraft Team и участники PdfCraft. Авторские уведомления сохранены в [NOTICE](NOTICE), происхождение ресурсов — в [ATTRIBUTION.md](ATTRIBUTION.md).

PdfCraft RU — независимый форк; его оформление не означает поддержку или одобрение со стороны ArtCraft. Название и товарные знаки ArtCraft не входят в открытую лицензию исходного кода. Подробная документация оригинального проекта доступна в [upstream README](https://github.com/storytold/pdfcraft#readme).
