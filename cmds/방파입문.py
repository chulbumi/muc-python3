# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['직위'] != '방주' and ob['직위'] != '부방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
            
        if line == '':
            ob.sendLine('☞ 사용법 : [대상] 방파입문')
            return

        obj = ob.env.findObjName(line)
        if obj == None or is_player(obj) == False:
            ob.sendLine('☞ 이곳에 그런 무림인이 없습니다.')
            return
        if obj == ob:
            ob.sendLine('☞ 자기 자신입니다.')
            return

        if ob.checkAttr('입문신청자', obj['이름']) == False:
            ob.sendLine('☞ 방파를 신청한 그런 무림인이 없습니다.')
            return
        ob.delAttr('입문신청자', obj['이름'])
        
        ob.sendLine('당신이 %s 방파에 입문시켰음을 선포합니다.' % obj.han_obj())
        obj.sendLine('\r\n%s 당신을 방파에 입문시켰음을 선포합니다.' % ob.han_iga())
        obj.lpPrompt()
        obj['소속'] = ob['소속']
        obj['직위'] = '방파인'
        g = GUILD[obj['소속']]
        if '방파인리스트' not in g:
            g['방파인리스트'] = [obj['이름']]
        else:
            g['방파인리스트'].append(obj['이름'])
        g['방파원수'] += 1
        GUILD.save()
        #방파원들에게도 알려야함!!
