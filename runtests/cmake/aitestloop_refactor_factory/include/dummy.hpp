#pragma once
#include <optional>

class DummyFactory;

class Dummy {
    friend class DummyFactory;
    Dummy() = default;

public:
    int value() const { return 1; }
};

class DummyFactory {
public:
    static std::optional<Dummy> create(bool allowed) {
        if (!allowed) return std::nullopt;
        return Dummy{};
    }
};
