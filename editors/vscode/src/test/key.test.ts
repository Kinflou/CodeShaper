import assert from "node:assert/strict";
import { test } from "node:test";
import { Clock, HeldKey } from "../explore/key";

interface Timer {
    at: number;
    run: () => void;
}

class Time implements Clock<Timer> {
    private time = 1_000_000;
    private timers: Timer[] = [];

    now(): number {
        return this.time;
    }

    after(milliseconds: number, run: () => void): Timer {
        const timer = { at: this.time + milliseconds, run };
        this.timers.push(timer);

        return timer;
    }

    cancel(timer: Timer): void {
        this.timers = this.timers.filter(other => other !== timer);
    }

    wait(milliseconds: number): void {
        const end = this.time + milliseconds;

        for (;;) {
            const next = this.timers.filter(timer => timer.at <= end).sort((one, other) => one.at - other.at)[0];
            if (!next) {
                break;
            }

            this.time = next.at;
            this.cancel(next);
            next.run();
        }
        this.time = end;
    }
}

function setup() {
    const time = new Time();
    const light = { isOn: false, turn(isOn: boolean) { this.isOn = isOn; } };

    return { time, light, key: new HeldKey(light, time) };
}

test("a tap turns the hints on and off", () => {
    const { time, light, key } = setup();

    key.press();
    time.wait(60_000);
    assert.equal(light.isOn, true);

    key.press();
    time.wait(1_000);
    assert.equal(light.isOn, false);
});

test("two quick taps leave them off", () => {
    const { time, light, key } = setup();

    key.press();
    time.wait(200);
    key.press();
    time.wait(1_000);
    assert.equal(light.isOn, false);
});

for (const [delay, rate, jitter] of [[250, 30, 0], [500, 30, 0], [600, 40, 0], [720, 40, 0], [1000, 40, 0], [600, 40, 260], [600, 100, 0]]) {
    test(`holding shows them until released, ${delay} then every ${rate} to ${rate + jitter} ms`, () => {
        for (const wasOn of [false, true]) {
            const { time, light, key } = setup();
            if (wasOn) {
                key.press();
                time.wait(5_000);
            }

            key.press();
            time.wait(delay);
            for (let presses = 0; presses < 100; presses += 1) {
                key.press();
                if (presses > 2) {
                    assert.equal(light.isOn, true);
                }
                time.wait(rate + (presses % 2) * jitter);
                if (presses > 2) {
                    assert.equal(light.isOn, true);
                }
            }

            time.wait(1_500);
            assert.equal(light.isOn, false);
        }
    });
}

test("nothing happens after it's disposed", () => {
    const { time, light, key } = setup();

    key.press();
    time.wait(5_000);
    key.press();
    key.dispose();
    time.wait(5_000);
    assert.equal(light.isOn, true);
});
