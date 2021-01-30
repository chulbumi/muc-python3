# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *

class CmdObj(Command):
    level=2000
    def cmd(self, ob, line):
        if getInt(ob['관리자등급']) < 1000:
            ob.sendLine('☞ 무슨 말인지 모르겠어요. *^_^')
            return
        ob.write('방 설명 작성을 마치시려면 \'.\' 를 입력하세요.\r\n:')
        ob.INTERACTIVE = 0
        ob._lineData = ''
        ob._lineDataTarget = ob.env
        ob._lineDataValue = '설명'
        ob.input_to(ob.write_lines)

