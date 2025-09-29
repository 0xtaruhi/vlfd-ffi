#include <chrono>
#include <cstring>
#include <iostream>
#include <thread>

extern "C" {
#include "../../vlfd_ffi.h"
}

static const char* event_kind_name(VlfdHotplugEventKind kind) {
    switch (kind) {
        case Arrived:
            return "arrived";
        case Left:
            return "left";
        default:
            return "unknown";
    }
}

static void print_optional_u16(const char* label, VlfdOptionalU16 value) {
    if (value.has_value) {
        std::cout << "  " << label << ": 0x" << std::hex << value.value << std::dec << "\n";
    }
}

static void print_optional_u8(const char* label, VlfdOptionalU8 value) {
    if (value.has_value) {
        std::cout << "  " << label << ": 0x" << std::hex << static_cast<int>(value.value)
                  << std::dec << "\n";
    }
}

static void hotplug_callback(void* /*user_data*/, const VlfdHotplugEvent* event) {
    if (!event) {
        return;
    }

    std::cout << "Hotplug event: " << event_kind_name(event->kind) << "\n";
    std::cout << "  bus: " << static_cast<int>(event->device.bus_number)
              << ", address: " << static_cast<int>(event->device.address) << "\n";

    const VlfdSliceU8& ports = event->device.port_numbers;
    if (ports.len > 0 && ports.data != nullptr) {
        std::cout << "  ports:";
        for (size_t i = 0; i < ports.len; ++i) {
            std::cout << ' ' << static_cast<int>(ports.data[i]);
        }
        std::cout << "\n";
    }

    print_optional_u16("vendor", event->device.vendor_id);
    print_optional_u16("product", event->device.product_id);
    print_optional_u8("class", event->device.class_code);
    print_optional_u8("subclass", event->device.sub_class_code);
    print_optional_u8("protocol", event->device.protocol_code);
    std::cout.flush();
}

static void print_last_error(const char* prefix) {
    const char* msg = vlfd_get_last_error_message();
    if (msg && std::strlen(msg) > 0) {
        std::cout << prefix << ": " << msg << "\n";
    } else {
        std::cout << prefix << ": (no error message)\n";
    }
}

int main() {
    VlfdHotplugOptions options = vlfd_hotplug_options_default();
    options.enumerate_existing = true;

    VlfdHotplugRegistration* registration =
        vlfd_hotplug_register(&options, hotplug_callback, nullptr);
    if (!registration) {
        print_last_error("Failed to register hotplug callback");
        return 1;
    }

    std::cout << "Hotplug callback registered; waiting for events..." << std::endl;

    std::this_thread::sleep_for(std::chrono::seconds(20));

    if (vlfd_hotplug_unregister(registration) != 0) {
        print_last_error("Failed to unregister hotplug callback");
    } else {
        std::cout << "Hotplug callback unregistered." << std::endl;
    }

    VlfdDevice* device = vlfd_io_open();
    if (!device) {
        print_last_error("Could not open device (expected if hardware is absent)");
    } else {
        std::cout << "Device opened successfully." << std::endl;
        vlfd_io_close(device);
    }

    return 0;
}
