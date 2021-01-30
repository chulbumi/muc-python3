# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['소속'] != '':
            ob.sendLine('☞ 방파에 입문 신청을 할 수 없습니다.')
            return
            
        if line == '':
            ob.sendLine('☞ 사용법 : [방주이름] 입문신청')
            return

        obj = ob.env.findObjName(line)
        if obj == None  or is_player(obj) == False:
            ob.sendLine('☞ 이곳에 그런 무림인이 없습니다.')
            return
        if obj == ob:
            ob.sendLine('☞ 자기 자신입니다.')
            return
        if obj['직위'] != '방주' and obj['직위'] != '부방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
        if obj['성격'] == '기인':
            if ob['성격'] != '기인' and ob['성격'] != '선인' and ob['성격'] != obj['기존성격']:
                ob.sendLine('☞ 방파에 입문 신청을 할 수 없습니다.')
                return
        elif obj['성격'] == '선인' and ob['성격'] == '':
            ob.sendLine('☞ 방파에 입문 신청을 할 수 없습니다.')
            return
        else:
             if ob['성격'] != '기인' and ob['성격'] != '선인' and obj['성격'] != ob['성격']:
                ob.sendLine('☞ 방파에 입문 신청을 할 수 없습니다.')
                return
        if obj.checkAttr('입문신청자', ob['이름']):
            ob.sendLine('☞ 이미 입문 신청을 하였습니다.')
            return
        obj.setAttr('입문신청자', ob['이름'])
        ob.sendLine('당신이 %s의 방파에 입문을 신청합니다.' % obj.getNameA())
        obj.sendLine('\r\n%s 당신의 방파에 입문을 신청합니다.' % ob.han_iga())
        obj.lpPrompt()
