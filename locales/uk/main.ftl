## Командний рядок

cli-about = Консольний редактор файлу hosts
cli-file = Редагувати цей файл замість hosts із конфігу
cli-lang = Мова інтерфейсу: uk або en
cli-help = Показати цю довідку
cli-version = Показати версію
cli-usage = Використання
cli-options = Параметри
cli-commands = Команди
cli-arguments = Аргументи
cli-dry-run = Показати зміни як diff, нічого не зберігаючи
cli-quiet = Нічого не виводити, якщо все вдалося
cmd-list = Показати записи за групами
cmd-groups = Показати групи з кількістю увімкнених записів
cmd-on = Увімкнути групи або хости
cmd-off = Вимкнути групи або хости
cmd-toggle = Перемкнути групи або хости в протилежний стан
cmd-add = Додати запис
cmd-rm = Видалити імена хостів або, з --group, цілі групи
cmd-completions = Вивести скрипт автодоповнення для shell
arg-list-group = Лише ця група
arg-enabled = Лише увімкнені записи
arg-disabled = Лише вимкнені записи
arg-json = Вивести JSON
arg-targets = Назви груп або хостів
arg-as-group = Вважати назви групами
arg-as-host = Вважати назви хостами
arg-force = Дозволити змінювати системні записи (localhost тощо)
arg-ip = IP-адреса
arg-hosts = Імена хостів
arg-add-group = Додати в цю групу (створюється, якщо її немає)
arg-comment = Опис
arg-add-disabled = Додати запис вимкненим
arg-rm-names = Імена хостів (з --group — назви груп)
arg-rm-group = Видалити названі групи разом із записами
arg-keep-entries = Залишити записи групи, видалити лише саму групу
arg-shell = Shell: bash, zsh, fish, elvish або powershell
cli-entries = { $count ->
    [one] { $count } запис
    [few] { $count } записи
   *[many] { $count } записів
}
cli-group-on = Увімкнено групу «{ $name }» ({ cli-entries })
cli-group-off = Вимкнено групу «{ $name }» ({ cli-entries })
cli-group-already-on = Група «{ $name }» вже увімкнена
cli-group-already-off = Група «{ $name }» вже вимкнена
cli-host-on = Увімкнено { $entry }
cli-host-off = Вимкнено { $entry }
cli-host-already-on = { $host } вже увімкнено
cli-host-already-off = { $host } вже вимкнено
cli-added = Додано { $entry }
cli-added-to = Додано { $entry } у «{ $group }»
cli-add-exists = { $entry } вже є у файлі
cli-removed-host = Видалено { $host } із { $entry }
cli-removed-entry = Видалено { $entry }
cli-group-removed = Видалено групу «{ $name }» ({ cli-entries })
cli-group-unwrapped = Видалено групу «{ $name }»; { cli-entries } залишено без групи
cli-warn-conflict = Увага: { $host } тепер увімкнено з різними адресами: { $ips }
cli-error-not-found = Немає групи чи хоста «{ $name }». Див.: hostkeep list
cli-error-no-group = Немає групи «{ $name }». Див.: hostkeep groups
cli-error-no-host = Немає хоста «{ $name }». Див.: hostkeep list
cli-error-rm-group-hint = Хоста «{ $name }» немає, але є така група. Щоб видалити групу: hostkeep rm --group "{ $name }"
cli-error-ambiguous = «{ $name }» — і група, і хост. Додайте --group або --host.
cli-error-host-many = { $host } є в кількох записах; якщо увімкнути всі, вони конфліктуватимуть. Перемкніть групу з потрібним записом або відредагуйте в TUI:
cli-error-protected-off = { $host } — системний запис; його вимкнення може зламати систему. Повторіть із --force.
cli-error-protected-rm = { $host } — системний запис; його видалення може зламати систему. Повторіть із --force.
cli-error-protected-group = Група «{ $name }» містить системний запис (localhost тощо). Повторіть із --force або використайте --keep-entries.
cli-error-sudo-no-tty = Для запису { $path } потрібні права адміністратора, але немає терміналу, щоб запитати пароль. Запустіть команду через sudo.
cli-error-sudo-failed = Не збережено: sudo не вдався.
error-config = Не вдалося прочитати конфіг { $path }: { $error }. Використано типові налаштування.
error-not-terminal = hostkeep потребує інтерактивного терміналу.
error-open = Не вдалося відкрити { $path }: { $error }
sudo-prompt = Для запису { $path } потрібні права адміністратора.

## Заголовок і панелі

header-modified = змінено
header-needs-sudo = для збереження потрібен sudo
panel-groups = Групи
group-all = Усі
group-ungrouped = Без групи
enabled-of = { $enabled }/{ $total } увімкнено
entries-count = { $count ->
    [one] { $count } запис
    [few] { $count } записи
   *[many] { $count } записів
}
col-ip = IP
col-hosts = Хости
col-comment = Опис
col-group = Група
empty-group = Записів ще немає — натисніть a, щоб додати
empty-filter = Нічого не знайдено

## Нижній рядок

key-toggle = вкл/викл
key-toggle-group = вкл/викл групу
key-add = додати
key-edit = ред.
key-delete = видал.
key-search = пошук
key-save = зберегти
key-help = довідка
key-new-group = нова група
key-rename = перейм.
key-move = перемістити
key-apply = застосувати
key-clear = скинути
key-scroll = прокрутка
key-next-field = наступне поле

## Рядок статусу

status-saved = Збережено, бекап: { $backup }
status-dns-flushed = DNS-кеш скинуто
status-dns-failed = не вдалося скинути DNS-кеш
status-no-changes = Немає змін для збереження
status-save-failed = Не вдалося зберегти: { $error }
status-sudo-failed = Не збережено: sudo не вдався. Ваші зміни на місці.
status-backup-failed = Не вдалося створити бекап, нічого не записано: { $error }
status-error = Помилка: { $error }
status-undo = Скасовано
status-redo = Повторено
status-nothing-to-undo = Нічого скасовувати
status-nothing-to-redo = Нічого повторювати
status-reloaded = Перезавантажено з диска
status-restored = Відновлено з { $name } — натисніть s, щоб зберегти
status-no-backups = Бекапів ще немає
status-language = Мова: українська
status-select-group = Виберіть групу, щоб її перемкнути
status-group-builtin = Це вбудована група, її не можна змінити
status-marked = { $count ->
    [one] Вибрано { $count } запис
    [few] Вибрано { $count } записи
   *[many] Вибрано { $count } записів
} — Space, d, m діють на всі, Esc скидає
warn-conflict = { $host } увімкнено з кількома IP: { $ips }
warn-duplicate = Цей запис дублює один із попередніх
warn-conflicts-total = { $count ->
    [one] { $count } хост увімкнено з кількома IP
    [few] { $count } хости увімкнено з кількома IP
   *[many] { $count } хостів увімкнено з кількома IP
}
warn-invalid-lines = { $count ->
    [one] { $count } рядок не є коректним записом (див. r)
    [few] { $count } рядки не є коректними записами (див. r)
   *[many] { $count } рядків не є коректними записами (див. r)
}

## Форма

form-add-title = Новий запис
form-edit-title = Редагування запису
form-ip = IP-адреса
form-hosts = Хости
form-comment = Опис
form-group = Група
form-state = Стан
form-enabled = увімкнено
error-ip-empty = Введіть IP-адресу
error-ip-invalid = "{ $ip }" — некоректна IP-адреса
error-hosts-empty = Введіть хоча б одне ім'я хоста
error-host-invalid = "{ $host }" — некоректне ім'я хоста
group-new-title = Нова група
group-rename-title = Перейменувати групу
error-group-empty = Введіть назву групи
error-group-exists = Група "{ $name }" вже існує
picker-title = Перемістити в групу

## Діалоги

btn-yes = Так
btn-ok = OK
btn-cancel = Скасувати
btn-save = Зберегти
btn-back = Назад
btn-close = Закрити
btn-delete = Видалити
btn-discard = Не зберігати
btn-reload = Перезавантажити
btn-overwrite = Перезаписати
btn-restore = Відновити
btn-delete-with-entries = Видалити з записами
btn-keep-entries = Залишити записи
dialog-delete-title = Видалення
dialog-delete-entries = Видалити { $count ->
    [one] { $count } запис
    [few] { $count } записи
   *[many] { $count } записів
}?
dialog-delete-group = Видалити групу "{ $name }" ({ $count ->
    [one] { $count } запис
    [few] { $count } записи
   *[many] { $count } записів
})? Записи можна видалити разом із нею або залишити без групи.
dialog-protected-title = Системний запис
dialog-protected-toggle = Серед вибраних є системний запис (localhost тощо). Його вимкнення може зламати систему. Продовжити?
dialog-protected-delete = Серед вибраних є системний запис (localhost тощо). Його видалення може зламати систему. Все одно видалити?
dialog-quit-title = Незбережені зміни
dialog-quit = Зберегти зміни перед виходом?
dialog-external-title = Файл змінено на диску
dialog-external = Файл hosts змінила інша програма після відкриття. Перезавантажити його (ваші зміни лишаться в undo) чи перезаписати?

## Diff, сирий файл, бекапи

diff-title = Зберегти зміни в { $path }?
diff-backup-note = Буде створено бекап.
diff-sudo-note = Буде потрібен sudo.
raw-title = Сирий файл (тільки читання)
backups-title = Бекапи
backups-diff-title = Поточний → бекап
backups-identical = Збігається з поточним станом

## Довідка

help-title = Клавіші
help-move = Вгору / вниз
help-panels = Перемкнути панель
help-jump = На початок / кінець, сторінка вгору / вниз
help-toggle = Увімкнути / вимкнути запис або групу
help-add = Додати запис (у панелі груп — групу)
help-edit = Редагувати запис, перейменувати групу
help-delete = Видалити
help-duplicate = Дублювати запис
help-move-group = Перемістити запис в іншу групу
help-reorder = Посунути запис або групу вгору / вниз
help-mark = Вибрати кілька записів
help-search = Пошук
help-undo = Скасувати / повторити
help-save = Зберегти (з переглядом змін)
help-raw = Переглянути сирий файл
help-backups = Бекапи
help-language = Змінити мову (uk ⇄ en)
help-help = Ця довідка
help-quit = Вихід
