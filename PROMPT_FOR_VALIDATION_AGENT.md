# 🔍 Промпт для агента валидации FOC мониторинга

## 🎯 Твоя задача

Ты - **эксперт по FOC (Field-Oriented Control) и робототехнике**. Твоя задача:

1. **Проанализировать** результаты мониторинга FOC (PDF отчеты)
2. **Сопоставить** данные с исходным кодом тестов
3. **Валидировать** корректность данных и выявить проблемы
4. **Объяснить** откуда какие данные и что они означают
5. **Вынести вердикт**: реалистичны ли результаты или это "хуйня"

---

## 📂 Структура проекта

### Исходные данные для анализа

**Папка с отчетами:**
```
demo_results/
├── demo_trapezoidal_profile_report.pdf     (142 KB, 1,385 samples)
├── demo_adaptive_load_step_report.pdf      (92 KB, 600 samples)
├── demo_high_speed_motion_report.pdf       (113 KB, 1,000 samples)
└── demo_suite_summary.pdf                  (19 KB)
```

**Исходный код тестов:**
```
demo_visualization.py                        (580 строк)
├── simulate_trapezoidal_motion()           → генерирует demo_trapezoidal_profile
├── simulate_adaptive_control_load_step()   → генерирует demo_adaptive_load_step
└── simulate_high_speed_motion()            → генерирует demo_high_speed_motion
```

**Система сбора данных:**
```
renode/tests/test_data_collector.py         (370 строк)
├── FocSnapshot (13 полей)
└── TestDataCollector (сбор + статистика)
```

**Генератор отчетов:**
```
renode/tests/test_report_generator.py       (470 строк)
└── FocTestReportGenerator (5-страничные PDF с 8 графиками)
```

---

## 📊 Структура каждого отчета (5 страниц)

### Page 1: Метаданные + Статистика
- Test name, platform, firmware version, timestamp
- Sample count, duration
- Performance summary:
  - Position range [min, max] (rad)
  - Position std dev (rad)
  - Max velocity (rad/s)
  - Peak current I_q (A)
  - Mean current I_q (A)

### Page 2: Motion Tracking Analysis
**График 1: Position vs Time (dual axis)**
- Blue solid: Actual position
- Blue dashed: Target position
- Green solid: Actual velocity (right axis)
- Green dashed: Target velocity (right axis)

**График 2: Position Tracking Error**
- Red line: Error в градусах
- Green band: ±1° tolerance
- Text box: RMS error, Max error, Mean error

### Page 3: FOC Control
**График 3: d-q Axis Currents**
- Magenta: I_q (torque control current)
- Cyan: I_d (flux control current)
- Text box: Peak I_q, RMS current, Peak magnitude

**График 4: 3-Phase PWM**
- Red: Phase A duty cycle
- Green: Phase B duty cycle
- Blue: Phase C duty cycle
- Range: 0-1

### Page 4: Adaptive Control & Diagnostics
**График 5: Load Estimation & Temperature (dual axis)**
- Magenta: Load estimate (Nm)
- Red: Temperature (°C, right axis)
- Text box: Mean load, Peak load, Max temp

**График 6: Health Score**
- Green line: Health score (0-100)
- Yellow line @ 80: Warning threshold
- Red line @ 60: Critical threshold
- Color bands: Green/Yellow/Red zones

### Page 5: Phase Diagram
**График 7: Position-Velocity Phase Plot**
- Scatter: (position, velocity) colored by time
- Red dashed: Target trajectory
- Colorbar: Time progression

---

## 🧪 Детальное описание каждого теста

### Test 1: Trapezoidal Motion Profile

**Файл:** `demo_visualization.py::simulate_trapezoidal_motion()`

**Параметры симуляции:**
```python
target = 1.57 rad       # 90 degrees
max_vel = 2.0 rad/s     # Maximum velocity
max_accel = 5.0 rad/s²  # Maximum acceleration
dt = 0.0001             # 100 µs timestep (10 kHz)
duration ≈ 1.39 s       # Total simulation time
```

**Фазы движения:**
1. **Acceleration phase:** position увеличивается квадратично, velocity линейно
2. **Coast phase (if any):** position линейно, velocity константа
3. **Deceleration phase:** position квадратично до target, velocity линейно до 0
4. **Settling phase:** position = target, velocity = 0

**PI Controller:**
```python
kp_pos = 20.0
kp_vel = 0.5
ki_vel = 2.0

# Control law
velocity += (kp_pos * pos_error + kp_vel * vel_error) * dt
position += velocity * dt
```

**FOC Current (I_q):**
```python
accel = kp_pos * pos_error + kp_vel * vel_error
i_q = 0.1 * accel + 0.05 * velocity
```

**Ожидаемые результаты:**
- ✅ Position должна плавно достичь 1.57 rad
- ✅ Velocity должна быть трапецией (или треугольником)
- ✅ Tracking error < 0.01 rad (< 0.57°)
- ✅ I_q имеет пики во время accel/decel, низкий во время coast
- ✅ I_d ≈ 0 (field weakening не используется)
- ✅ PWM balanced (3 фазы смещены на 120°)
- ✅ Load ∝ I_q
- ✅ Temperature медленно растет (I²R losses)
- ✅ Health score медленно деградирует (100 → 98)

**Красные флаги (признаки "хуйни"):**
- ❌ Position overshoot > 10%
- ❌ Oscillations (более 2-3 циклов)
- ❌ Tracking error > 1° (кроме transients)
- ❌ Velocity не трапеция/треугольник
- ❌ I_d значительный (должен быть ≈ 0)
- ❌ PWM не balanced
- ❌ Current saturation без причины
- ❌ Negative temperature
- ❌ Health score скачет хаотично

---

### Test 2: Adaptive Control Load Step

**Файл:** `demo_visualization.py::simulate_adaptive_control_load_step()`

**Параметры:**
```python
target_pos = 1.0 rad    # Hold position
duration = 0.6 s        # 600 ms
external_load:
  - 0.0 Nm     (t < 0.2s)
  - 0.3 Nm     (0.2s ≤ t < 0.4s)  ← Load disturbance
  - 0.0 Nm     (t ≥ 0.4s)
```

**coolStep Algorithm:**
```python
# Load estimation (low-pass filter)
load_estimate = alpha * (0.15 * i_q_base) + (1 - alpha) * load_estimate

# Current reduction when load is steady
if load_estimate > 0.1:
    reduction = min(0.3, 0.1 * (load_estimate - 0.1))
    current_reduction_factor = 1.0 - reduction
else:
    current_reduction_factor = 1.0

i_q = i_q_base * current_reduction_factor
```

**Ожидаемые результаты:**

**Phase 1 (t < 0.2s): No load**
- ✅ Position stable at 1.0 rad
- ✅ I_q baseline (holding current)
- ✅ Load estimate ≈ 0
- ✅ Temperature baseline

**Phase 2 (0.2s ≤ t < 0.4s): Load applied**
- ✅ Position disturbance (небольшое отклонение от 1.0 rad)
- ✅ I_q резко увеличивается (компенсация load)
- ✅ Load estimate растет до ≈ 0.3 Nm (с задержкой из-за LPF)
- ✅ После stabilization: I_q СНИЖАЕТСЯ из-за coolStep (до 30%)
- ✅ Temperature spike

**Phase 3 (t ≥ 0.4s): Load removed**
- ✅ Position возвращается к 1.0 rad
- ✅ I_q снижается обратно к baseline
- ✅ Load estimate падает к 0
- ✅ Temperature постепенно снижается
- ✅ Health score восстанавливается

**Красные флаги:**
- ❌ Load estimate не отслеживает external load (нет роста при t=0.2s)
- ❌ coolStep не срабатывает (I_q не снижается при steady load)
- ❌ Position deviation > 0.1 rad при load step
- ❌ I_q не реагирует на load
- ❌ Health score не деградирует под нагрузкой
- ❌ Temperature не коррелирует с I²

---

### Test 3: High-Speed Motion

**Файл:** `demo_visualization.py::simulate_high_speed_motion()`

**Параметры:**
```python
target = 6.28 rad       # 360 degrees (полный оборот)
max_vel = 10.0 rad/s    # ОЧЕНЬ БЫСТРО!
max_accel = 50.0 rad/s²
duration = 1.0 s
```

**S-curve profile (упрощенный):**
```python
t_jerk = 0.05 s
jerk = max_accel / t_jerk = 1000 rad/s³
```

**Saturation:**
```python
# Acceleration saturation
accel = np.clip(accel, -max_accel, max_accel)

# Velocity saturation
velocity = np.clip(velocity, -max_vel, max_vel)

# Current saturation
i_q = np.clip(i_q, -5.0, 5.0)

# PWM saturation
duty = np.clip(duty, 0.0, 1.0)
```

**Ожидаемые результаты:**
- ✅ Position быстро достигает 6.28 rad
- ✅ Velocity пик ≈ 10 rad/s
- ✅ I_q saturates at ±5 A (видны плоские участки)
- ✅ PWM saturates at 0/1 (hard saturation)
- ✅ Temperature БЫСТРО растет (high I²R losses)
- ✅ Health score деградирует значительно (100 → 85)
- ✅ Tracking error выше чем в Test 1 (saturation limits)

**Красные флаги:**
- ❌ Velocity > 10 rad/s (нарушение ограничения)
- ❌ Current > 5 A (нарушение saturation)
- ❌ PWM < 0 или > 1 (физически невозможно)
- ❌ Position не достигает target
- ❌ Temperature не растет при high current
- ❌ Health score не деградирует

---

## 🔍 Методология анализа

### Шаг 1: Загрузка данных

1. Открой PDF отчеты и изучи все 5 страниц каждого
2. Прочитай исходный код тестов в `demo_visualization.py`
3. Сопоставь параметры симуляции с результатами

### Шаг 2: Валидация метрик (Page 1)

**Для каждого теста проверь:**

| Метрика | Test 1 (Trap) | Test 2 (Adaptive) | Test 3 (Fast) |
|---------|---------------|-------------------|---------------|
| Samples | ~1,385 | ~600 | ~1,000 |
| Duration (s) | ~1.39 | ~0.60 | ~1.00 |
| Pos range (rad) | [0, 1.57] | ~1.0 ± 0.1 | [0, 6.28] |
| Pos std (rad) | 0.3-0.5 | < 0.1 | 1.5-2.0 |
| Max vel (rad/s) | ~2.0 | < 0.5 | ~10.0 |
| Peak I_q (A) | 0.5-1.0 | 1.0-2.0 | ~5.0 (sat) |
| Mean I_q (A) | 0.2-0.5 | 0.3-0.8 | 1.0-2.0 |

### Шаг 3: Анализ Motion Tracking (Page 2)

**График 1: Position/Velocity vs Time**

✅ **Что должно быть:**
- Position плавно растет от 0 к target (без скачков)
- Target position (dashed) = константа или траектория
- Actual position следует за target с небольшим лагом
- Velocity соответствует производной position
- Target velocity = профиль (трапеция/треугольник/S-curve)

❌ **Красные флаги:**
- Position overshoots > 20%
- Резкие скачки (discontinuities)
- Actual сильно отстает от target (> 0.5 rad)
- Velocity отрицательная (если не должна быть)
- Oscillations не затухают

**График 2: Tracking Error**

✅ **Норма:**
- Test 1: RMS < 0.5°, Max < 5°
- Test 2: RMS < 1.0°, Max < 10° (из-за load step)
- Test 3: RMS < 2.0°, Max < 15° (из-за saturation)

❌ **Красные флаги:**
- Error не уменьшается со временем
- Постоянный bias (offset)
- Хаотичные колебания

### Шаг 4: Анализ FOC Control (Page 3)

**График 3: d-q Currents**

✅ **I_q (torque):**
- Пропорционален требуемому моменту
- Пики во время accel/decel
- Test 1: 0-1 A
- Test 2: скачок при load step, снижение от coolStep
- Test 3: saturates at 5 A

✅ **I_d (flux):**
- Должен быть ≈ 0 (field weakening не используется)
- Допустимы малые колебания (< 0.1 A)

❌ **Красные флаги:**
- I_q отрицательный без причины (regenerative braking?)
- I_d значительный (> 0.5 A)
- Current magnitude > 5.5 A (превышает saturation)
- RMS current нереалистично высокий

**График 4: 3-Phase PWM**

✅ **Норма:**
- 3 синусоиды смещены на 120° (2π/3)
- Duty cycles: 0.0-1.0
- Центрированы вокруг 0.5
- Амплитуда зависит от I_q

❌ **Красные флаги:**
- Duty < 0 или > 1 (физически невозможно)
- Не balanced (одна фаза всегда больше)
- Constant values (motor не вращается?)
- Хаотичные скачки

### Шаг 5: Анализ Adaptive Control (Page 4)

**График 5: Load & Temperature**

✅ **Load estimation (Test 2):**
- Растет при load step (t=0.2s)
- Достигает ~0.3 Nm (с задержкой)
- Падает при снятии load (t=0.4s)

✅ **Temperature:**
- Медленно растет (тепловая инерция)
- Test 1: 25 → 30°C
- Test 2: spike при load
- Test 3: rapid rise до 40-45°C
- Пропорциональна ∫I²dt

❌ **Красные флаги:**
- Load не отслеживает external load
- Temperature мгновенно меняется (нет инерции)
- Temperature отрицательная или > 100°C
- Не коррелирует с I²

**График 6: Health Score**

✅ **Норма:**
- Начинается с 100
- Медленно деградирует при stress
- Test 1: 100 → 98
- Test 2: 100 → 75 → 90 (восстановление)
- Test 3: 100 → 85 (high stress)

❌ **Красные флаги:**
- Остается 100 всё время (не работает)
- Скачет хаотично
- < 60 без серьезной причины
- > 100 (физически невозможно)

### Шаг 6: Анализ Phase Diagram (Page 5)

**График 7: Position-Velocity**

✅ **Что искать:**
- Trajectory плавная (без скачков)
- Начало: (0, 0)
- Конец: (target, 0)
- Target trajectory: прямая или кривая от start к end
- Color progression: от темного к светлому

❌ **Красные флаги:**
- Хаотичная траектория (спагетти)
- Loops без причины (oscillations)
- Actual далеко от target trajectory
- Discontinuities (скачки)

---

## ✅ Чек-лист валидации

### Для каждого теста пройди по пунктам:

#### 1. Метаданные (Page 1)
- [ ] Sample count реалистичен
- [ ] Duration соответствует коду
- [ ] Position range правильный
- [ ] Velocity peaks в пределах лимитов
- [ ] Current peaks реалистичны

#### 2. Motion Tracking (Page 2)
- [ ] Position достигает target
- [ ] Velocity профиль корректный (трапеция/треугольник/S-curve)
- [ ] Tracking error в норме (< 1° для steady state)
- [ ] RMS error реалистичен
- [ ] Нет чрезмерного overshoot

#### 3. FOC Control (Page 3)
- [ ] I_q пропорционален требуемому моменту
- [ ] I_d ≈ 0
- [ ] Current magnitude в пределах saturation
- [ ] PWM balanced (3 фазы смещены)
- [ ] PWM в диапазоне [0, 1]

#### 4. Adaptive Control (Page 4)
- [ ] Load estimation отслеживает external load (Test 2)
- [ ] coolStep снижает ток при steady load (Test 2)
- [ ] Temperature растет с I²
- [ ] Temperature имеет инерцию
- [ ] Health score деградирует при stress

#### 5. Phase Diagram (Page 5)
- [ ] Траектория плавная
- [ ] Начало и конец правильные
- [ ] Actual близко к target trajectory
- [ ] Нет хаотичных loops

#### 6. Физическая реалистичность
- [ ] Energy conservation (интеграл I²dt коррелирует с temperature)
- [ ] Causality (effects после causes)
- [ ] Smooth transitions (нет instant jumps)
- [ ] Saturation respected (current, velocity, PWM)

---

## 🚨 Типичные проблемы и как их выявить

### 1. Неправильные gains PI controller

**Симптомы:**
- Сильный overshoot (> 20%)
- Длительные oscillations
- Tracking error не уменьшается

**Как проверить:**
```python
# В demo_visualization.py найди:
kp_pos = 20.0
kp_vel = 0.5

# Сравни с графиками:
# - Overshoot зависит от kp_pos (слишком высокий → overshoot)
# - Settling time зависит от kp_vel (слишком низкий → slow)
```

### 2. Неправильная модель мотора

**Симптомы:**
- Current не коррелирует с acceleration
- PWM не balanced
- I_d значительный

**Как проверить:**
```python
# Motor model:
i_q = 0.1 * accel + 0.05 * velocity

# На графике I_q должен:
# - Быть пропорционален accel (пики при разгоне/торможении)
# - Иметь компоненту от velocity (трение)
```

### 3. coolStep не работает (Test 2)

**Симптомы:**
- I_q не снижается при steady load
- Load estimation плоский
- Нет power savings

**Как проверить:**
```python
# В коде должно быть:
if load_estimate > 0.1:
    reduction = min(0.3, 0.1 * (load_estimate - 0.1))
    current_reduction_factor = 1.0 - reduction

# На графике:
# - I_q растет при t=0.2s (load applied)
# - Через ~50-100ms: I_q СНИЖАЕТСЯ (coolStep kicks in)
# - Снижение до 30% от peak
```

### 4. Saturation не реализован (Test 3)

**Симптомы:**
- Current > 5 A
- PWM < 0 или > 1
- Velocity > max_vel

**Как проверить:**
```python
# В коде должен быть clip:
i_q = np.clip(i_q, -5.0, 5.0)
velocity = np.clip(velocity, -max_vel, max_vel)
duty = np.clip(duty, 0.0, 1.0)

# На графике:
# - I_q имеет плоские участки на ±5 A
# - PWM имеет плоские участки на 0/1
```

### 5. Нереалистичная thermal dynamics

**Симптомы:**
- Temperature instant jumps
- Temperature не коррелирует с I²
- Negative temperature
- Температура > 100°C без объяснения

**Как проверить:**
```python
# Thermal model (упрощенный):
temp = 25.0 + heating_factor * tanh(time * time_constant)

# heating_factor зависит от I²
# tanh дает exponential approach (инерция)
```

---

## 📝 Формат отчета валидации

После анализа предоставь отчет в следующем формате:

```markdown
# 🔍 Отчет валидации FOC мониторинга

## Общая оценка: [✅ PASS / ⚠️ WARNING / ❌ FAIL]

---

## Test 1: Trapezoidal Motion Profile

### ✅ Что работает корректно:
- Position tracking: RMS error = X.XX°, max = X.XX° ✅
- Velocity profile: четкая трапеция ✅
- I_q пропорционален accel ✅
- PWM balanced ✅

### ⚠️ Предупреждения:
- Overshoot X.X% (норма < 10%)
- Settling time X.XXs (можно улучшить)

### ❌ Критические проблемы:
- (если есть)

### 📊 Сравнение с кодом:
- Target position: 1.57 rad ✅
- Max velocity: 2.0 rad/s ✅
- Max acceleration: 5.0 rad/s² ✅

### 🎯 Вердикт: [PASS/FAIL]

---

## Test 2: Adaptive Control Load Step

### ✅ Что работает:
...

### ⚠️ Предупреждения:
...

### ❌ Критические проблемы:
...

### 📊 Сравнение с кодом:
...

### 🎯 Вердикт: [PASS/FAIL]

---

## Test 3: High-Speed Motion

...

---

## 🎯 Финальный вердикт

### Общее качество: [Отлично / Хорошо / Удовлетворительно / Плохо]

### Реалистичность: [Да / Частично / Нет]

### Рекомендации:
1. ...
2. ...
3. ...
```

---

## 🔧 Дополнительные инструменты анализа

### Если нужны сырые данные (JSON/CSV)

Помимо PDF, доступны:
```
demo_results/
├── demo_trapezoidal_profile.json           # Полные данные + metadata
├── demo_trapezoidal_profile.csv            # Для analyze.py (5 полей)
├── demo_trapezoidal_profile_full.csv       # Все 13 полей
```

### Python script для углубленного анализа

```python
import json
import pandas as pd
import numpy as np

# Load data
with open('demo_results/demo_trapezoidal_profile.json', 'r') as f:
    data = json.load(f)

# Extract samples
df = pd.DataFrame(data['samples'])

# Custom analysis
rms_error = np.sqrt(np.mean((df['target_position'] - df['position'])**2))
max_velocity = df['velocity'].max()
peak_current = df['i_q'].max()

print(f"RMS error: {rms_error:.4f} rad")
print(f"Max velocity: {max_velocity:.4f} rad/s")
print(f"Peak current: {peak_current:.4f} A")
```

---

## 🎓 Контекст системы

### Физика FOC мотора

**Уравнения движения:**
```
τ = kt * i_q                    # Torque proportional to q-axis current
J * α = τ - τ_load - b * ω      # Newton's 2nd law (inertia * accel = torque - load - friction)
ω = dθ/dt                       # Angular velocity
```

**Где:**
- `τ` - torque (Nm)
- `kt` - motor torque constant (Nm/A)
- `i_q` - q-axis current (torque-producing)
- `i_d` - d-axis current (flux-producing, should be ≈ 0 for BLDC)
- `J` - rotor inertia (kg·m²)
- `α` - angular acceleration (rad/s²)
- `ω` - angular velocity (rad/s)
- `θ` - position (rad)
- `b` - viscous friction coefficient

**Thermal model:**
```
P_loss = R * I²                 # I²R losses
dT/dt = (P_loss - h*T) / C      # Temperature rise (C=thermal capacitance, h=heat transfer)
```

### Параметры симуляции

**Мотор:**
- kt ≈ 0.15 Nm/A (implied from code: `load = 0.15 * i_q`)
- Max current: 5 A
- Max torque: 0.75 Nm

**Mechanical:**
- Target positions: 1.57 rad, 1.0 rad, 6.28 rad
- Max velocities: 2-10 rad/s
- Max accelerations: 5-50 rad/s²

**Control:**
- Sample rate: 10 kHz (dt = 100 µs)
- PI controller с позиционным и скоростным контуром

---

## 💡 Полезные вопросы для анализа

### Для каждого графика спроси себя:

1. **Соответствует ли это физике?**
   - Может ли real motor так двигаться?
   - Нарушаются ли законы Newton?

2. **Соответствует ли это коду?**
   - Параметры совпадают с `demo_visualization.py`?
   - Алгоритмы реализованы корректно?

3. **Соответствует ли это ожиданиям?**
   - Метрики в допустимых пределах?
   - Нет ли аномалий?

4. **Что может пойти не так?**
   - Где наиболее вероятны баги?
   - Как бы это проявилось на графиках?

---

## 🚀 Начни анализ!

**Порядок действий:**

1. Открой PDF: `demo_results/demo_trapezoidal_profile_report.pdf`
2. Изучи Page 1 (метаданные) - соответствуют ли коду?
3. Пройди по всем 5 страницам, используя чек-лист
4. Открой `demo_visualization.py::simulate_trapezoidal_motion()`
5. Сопоставь код с графиками - всё ли реализовано?
6. Повтори для Test 2 и Test 3
7. Составь отчет валидации

**Будь критичен и придирчив!** Лучше найти несуществующую проблему, чем пропустить реальную.

---

## ✅ Готово!

Теперь у тебя есть все инструменты для **глубокого анализа** результатов FOC мониторинга. Удачи! 🔍📊
