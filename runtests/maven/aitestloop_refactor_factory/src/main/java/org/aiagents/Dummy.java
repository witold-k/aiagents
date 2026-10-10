package org.aiagents;

import java.util.Optional;

public final class Dummy {
    private Dummy() {}

    public int value() { return 1; }

    public static final class Factory {
        private Factory() {}

        public static Optional<Dummy> create(boolean allowed) {
            return allowed ? Optional.of(new Dummy()) : Optional.empty();
        }
    }
}
