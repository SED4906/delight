#include <cctype>
#include <filesystem>
#include <fstream>
#include <map>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <string>
#include <sstream>
#include <unistd.h>
#include <vector>

#include "unit.h"

enum unit_suffix {
  UNIT_HUH = -1,
  UNIT_TARGET,
  UNIT_MOUNT,
  UNIT_SERVICE,
  UNIT_SOCKET,
};

unit_suffix get_unit_suffix(const char *unit_name) {
  const char *suffix_start = strchr(unit_name, '.');
  if (!suffix_start)
    return UNIT_HUH;
  if (!strcmp(suffix_start, ".target"))
    return UNIT_TARGET;
  if (!strcmp(suffix_start, ".mount"))
    return UNIT_MOUNT;
  if (!strcmp(suffix_start, ".service"))
    return UNIT_SERVICE;
  if (!strcmp(suffix_start, ".socket"))
    return UNIT_SOCKET;
  return UNIT_HUH;
}

std::vector<std::string> unit_directories = {
    "/usr/lib/systemd/system/",
};

std::string find_unit_path(const char *unit_name) {
  for (std::string prefix : unit_directories) {
    std::string result = prefix;
    result += unit_name;
    if (!access(result.c_str(), F_OK))
      return result;
  }
  return "";
}

void in_place_trim(std::string& str) {
  str.erase(str.begin(), std::find_if(str.begin(), str.end(), [](unsigned char ch) {
    return !std::isspace(ch);
  }));
  str.erase(std::find_if(str.rbegin(), str.rend(), [](unsigned char ch) {
    return !std::isspace(ch);
  }).base(), str.end());
}

std::vector<std::string> split_by_whitespace(std::string str) {
  std::stringstream sstr(str);
  std::string word;
  std::vector<std::string> words;

  while (sstr >> word) {
    words.push_back(word);
  }
  return words;
}

std::multimap<std::string, std::string> read_unit(std::string unit_path) {
  std::multimap<std::string, std::string> keyvalues;
  std::ifstream file;
  std::string working_line;
  std::string line;
  file.open(unit_path, std::ifstream::in);
  bool line_continues = false;
  while (file.good()) {
    if (!line_continues) {
      if(working_line.contains("=")) {
        size_t position = working_line.find("=");
        std::string key = working_line.substr(0, position);
        in_place_trim(key);
        std::string value = working_line.substr(position+1);
        in_place_trim(value);
        if (value.empty()) keyvalues.erase(key);
        else keyvalues.insert(std::pair(key, value));
      }
      working_line = "";
    }
    std::getline(file, line);
    if(line.length() == 0 || line.front() == '#' || line.front() == ';') {
      line_continues = true;
      continue;
    }
    working_line += line;
    if(line.back() == '\\') {
      line_continues = true;
      line.back() = ' ';
    } else line_continues = false;
  }
  return keyvalues;
}

bool load_unit_requires(std::multimap<std::string, std::string> keyvalues) {
  std::pair<std::multimap<std::string, std::string>::iterator, std::multimap<std::string, std::string>::iterator> requires_range = keyvalues.equal_range("Requires");
  for(std::multimap<std::string, std::string>::iterator it=requires_range.first; it!=requires_range.second; ++it) {
    std::string required_units = it->second;
    std::vector<std::string> required_units_vec = split_by_whitespace(required_units);
    for(std::string required_unit : required_units_vec) {
      if (!load_unit(required_unit.c_str())) return false;
    }
  }
  return true;
}

void load_unit_wants(std::multimap<std::string, std::string> keyvalues) {
  std::pair<std::multimap<std::string, std::string>::iterator, std::multimap<std::string, std::string>::iterator> wants_range = keyvalues.equal_range("Wants");
  for(std::multimap<std::string, std::string>::iterator it=wants_range.first; it!=wants_range.second; ++it) {
    std::string wants_units = it->second;
    std::vector<std::string> wants_units_vec = split_by_whitespace(wants_units);
    for(std::string wants_unit : wants_units_vec) {
      load_unit(wants_unit.c_str());
    }
  }
}

bool load_unit(const char *unit_name) {
  printf("Loading unit %s\n", unit_name);
  unit_suffix suffix = get_unit_suffix(unit_name);
  std::string unit_path = find_unit_path(unit_name);
  if(unit_path.empty()) return false;
  std::multimap<std::string, std::string> keyvalues = read_unit(unit_path);
  load_unit_requires(keyvalues);
  load_unit_wants(keyvalues);
  switch (suffix) {
    case UNIT_TARGET: {
      std::string wants_name(unit_name);
      wants_name += ".wants";
      std::string wants_path(find_unit_path(wants_name.c_str()));
      for(auto entry : std::filesystem::directory_iterator(wants_path)) {
        load_unit(entry.path().filename().c_str());
      }
      break;
    };
    default: break;
  }
  return true;
}
