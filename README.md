# Парсер тендеров (rostender.info)

Небольшой парсер для сбора информации о тендерах с сайта rostender.info, написанный на Rust.

## Стек технологий

- **Язык:** Rust (edition 2024)
- **Асинхронность:** Tokio, Futures
- **HTTP клиент:** Reqwest (с поддержкой cookies и форм)
- **Парсинг:** Scraper
- **Сериализация:** Serde, Serde JSON, Postcard
- **Планировщик задач:** Chrono
- **Отправка уведомлений:** Lettre (email)
- **Уведомления ползователя:** Native-dialog (для Windows)

## Функциональность

- Парсинг тендеров с сайта rostender.info
- Асинхронная обработка запросов
- Отправка уведомлений по email (через Lettre)
- Нативные диалоговые окна для взаимодействия с пользователем

### Установка и запуск

1. Клонируйте репозиторий:
   ```bash
   git clone https://github.com/KvA2KLvA5T/tender_paresr.git
   cd tender_paresr
   ```
2. Соберите и запустите проект:
   ```bash
   cargo run --release
   ```
