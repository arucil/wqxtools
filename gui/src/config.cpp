#include "config.h"

static Config config;

Config &Config::instance() {
  return config;
}