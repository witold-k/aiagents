#include "dummy.hpp"
#include <cassert>

int main() {
    // Outdated test: use DummyFactory and handle successful/failed creation.
    Dummy dummy;
    assert(dummy.value() == 1);
}
