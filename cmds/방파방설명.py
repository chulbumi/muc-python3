# -*- coding: utf-8 -*-

from objs.cmd import Command
from lib.hangul import *

class CmdObj(Command):
    level=2000
    def cmd(self, ob, line):
        if ob['직위'] != '방주' and ob['직위'] != '부방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
        if line == '':
            ob.sendLine('☞ 사용법: [이름] 방파방이름')
            return
        if ob.env['방파주인'] == '':
            ob.sendLine('☞ 무림인은 아무곳에나 이름을 새기지 않는다네.')
            return
            
        if ob.env['방파주인'] != ob['소속']:
            ob.sendLine('☞ 무림인은 아무곳에나 이름을 새기지 않는다네.')
            return
        ob.write('방 설명 작성을 마치시려면 \'.\' 를 입력하세요.\r\n:')
        ob.INTERACTIVE = 0
        ob._lineData = ''
        ob._lineDataTarget = ob.env
        ob._lineDataValue = '설명'
        ob.input_to(ob.write_lines)

