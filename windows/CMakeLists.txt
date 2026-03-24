cmake_minimum_required(VERSION 3.14)
project(exam_paper_detector LANGUAGES CXX)

# Tell Flutter's generated plugin script which native library to bundle.
set(exam_paper_detector_bundled_libraries
  "${CMAKE_CURRENT_LIST_DIR}/exam_paper_detector.dll"
  PARENT_SCOPE
)

