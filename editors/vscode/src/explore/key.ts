const REPEATED = 500;
const TAP = 150;
const RELEASED = 400;

export interface Clock<T> {
    now(): number;
    after(milliseconds: number, run: () => void): T;
    cancel(timer: T): void;
}

export const CLOCK: Clock<NodeJS.Timeout> = {
    now: () => Date.now(),
    after: (milliseconds, run) => setTimeout(run, milliseconds),
    cancel: timer => clearTimeout(timer)
};

export interface Switch {
    readonly isOn: boolean;
    turn(isOn: boolean): void;
}

export class HeldKey<T> {
    private timer: T | undefined;
    private pressed = 0;
    private presses = 0;

    constructor(private readonly light: Switch, private readonly clock: Clock<T>) {}

    press(): void {
        const now = this.clock.now();
        const gap = now - this.pressed;

        this.pressed = now;
        this.presses = gap < REPEATED ? this.presses + 1 : 1;
        this.dispose();

        if (this.presses > 2) {
            this.light.turn(true);
            this.offAfter(Math.max(RELEASED, gap * 3));
        } else if (this.light.isOn) {
            this.offAfter(TAP);
        } else {
            this.light.turn(true);
        }
    }

    dispose(): void {
        if (this.timer !== undefined) {
            this.clock.cancel(this.timer);
            this.timer = undefined;
        }
    }

    private offAfter(milliseconds: number): void {
        this.timer = this.clock.after(milliseconds, () => this.light.turn(false));
    }
}
