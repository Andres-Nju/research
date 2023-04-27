from PyQt6.QtCore import QDateTime, Qt, QTimer, QStringListModel, QItemSelection, QModelIndex
from PyQt6.QtWidgets import (QApplication, QCheckBox, QComboBox, QDateTimeEdit, QListView,
                             QDial, QDialog, QGridLayout, QGroupBox, QHBoxLayout, QLabel, QLineEdit,
                             QPushButton, QRadioButton, QScrollBar, QSizePolicy, QSlider, QSpinBox, QStyleFactory,
                             QTableWidget, QTabWidget, QTextEdit, QPlainTextEdit,
                             QVBoxLayout, QWidget)
import subprocess
import os

class WidgetGallery(QDialog):
    def __init__(self, parent=None):
        super(WidgetGallery, self).__init__(parent)

        self.read_file()

        self.createTopLeftGroupBox()
        self.createTopRightGroupBox()

        mainLayout = QGridLayout()
        mainLayout.addWidget(self.topLeftGroupBox, 1, 0)
        mainLayout.addWidget(self.topRightGroupBox, 1, 1)
        mainLayout.setColumnStretch(0, 1)
        mainLayout.setColumnStretch(1, 4)
        self.setLayout(mainLayout)

        self.setWindowTitle("Styles")
        self.setFixedWidth(1200)
        self.setFixedHeight(500)

    def read_file(self):
        with open('results/hac.txt') as f:
            self.file_lines = f.read().strip().split('\n')
        self.pos_map = dict()
        dist, group, entry = 2, 0, 0
        for idx, line in enumerate(self.file_lines):
            if line[0] == 'd':
                dist, group, entry = int(line[:-1].split(' ')[-1]), 0, 0
            elif line[0] == '-':
                group, entry = group + 1, 0
            else:
                self.pos_map[idx] = (dist, group, entry, line)
                entry = entry + 1

    def on_selection(self, selected, deselected):
        # Get the newly selected index
        indexes = selected.indexes()
        if len(indexes) > 0:
            new_index = indexes[0].row()
            if new_index in self.pos_map:
                d, g, e, line = self.pos_map[new_index]
                self.pos_label.setText("Dist = %d, Group = %d, Entry = %d" % (d, g, e))
                name, commit = eval(line)
                path = os.path.join('results', 'File_Code', name, commit)
                dir_name = os.listdir(path)[0]
                path = os.path.join(path, dir_name)
                commit_message_file = os.path.join(path, 'commit_message.txt')
                with open(commit_message_file) as f:
                    commit_message = f.read()
                self.commit_message_label.setText(commit_message)
                before, after = os.path.join(path, dir_name + '_before.rs'), os.path.join(path, dir_name + '_after.rs')
                diff_output = subprocess.run(['diff', before, after, '-u'], stdout=subprocess.PIPE).stdout.decode()
                self.text_view.setPlainText(diff_output)
            else:
                self.commit_message_label.setText("")
                self.pos_label.setText("")

    def createTopLeftGroupBox(self):
        self.topLeftGroupBox = QGroupBox("Commits")

        self.pos_label = QLabel()

        model = QStringListModel(self.file_lines)
        view = QListView()
        view.setModel(model)
        view.selectionModel().selectionChanged.connect(self.on_selection)

        layout = QVBoxLayout()
        layout.addWidget(self.pos_label)
        layout.addWidget(view)

        layout.addStretch(1)
        self.topLeftGroupBox.setLayout(layout)

    def createTopRightGroupBox(self):
        self.topRightGroupBox = QGroupBox("Diff")

        self.commit_message_label = QLabel()

        self.text_view = QPlainTextEdit()
        doc = self.text_view.document()
        font = doc.defaultFont()
        font.setFamily("Courier New")
        doc.setDefaultFont(font)
        self.text_view.setPlainText("")

        layout = QVBoxLayout()
        layout.addWidget(self.text_view)
        layout.addWidget(self.commit_message_label)

        layout.addStretch(1)
        self.topRightGroupBox.setLayout(layout)


if __name__ == '__main__':
    import sys

    app = QApplication(sys.argv)
    gallery = WidgetGallery()
    gallery.show()
    sys.exit(app.exec())