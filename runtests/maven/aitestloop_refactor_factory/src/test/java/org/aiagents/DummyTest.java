package org.aiagents;

public final class DummyTest {
    public static void main(String[] args) {
        // Outdated test: use Dummy.Factory and handle Optional<Dummy>.
        Dummy dummy = new Dummy();
        if (dummy.value() != 1) throw new AssertionError("Expected 1");
    }
}
