# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):
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
        if len(line) > 20:
            ob.sendLine('☞ 너무 길어요.')
            return

        ob.env['이름'] = line
        ob.env.save()
        ob.sendLine('이름이 변경 되었습니다.')
        
