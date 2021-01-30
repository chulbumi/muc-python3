# -*- coding: utf-8 -*-

from objs.cmd import Command
from include.ansi import *

class CmdObj(Command):

    def cmd(self, ob, line):
        if line == '':
            ob.sendLine('☞ 사용법: [내용] 머리말')
            return
        if len(line) > 20:
            ob.sendLine('☞ 너무 깁니다.')
            return
        ob['머리말'] = line
        ob.sendLine('☞ 머리말을 설정 하였습니다.')
