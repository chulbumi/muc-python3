# -*- coding: utf-8 -*-

from objs.cmd import Command

class CmdObj(Command):

    def cmd(self, ob, line):
        if ob['직위'] != '방주':
            ob.sendLine('☞ 방파의 방주만이 할 수 있습니다.')
            return
        if line == '':
            ob.sendLine('☞ 사용법 : [대상] 방파파문')
            return

        if line == ob['이름']:
            ob.sendLine('☞ 자기 자신입니다.')
            return
        found = False
        for obj in ob.channel.players:
            if obj['이름'] == line:
                found = True
                break
        if found == False:
            obj = Player()
            obj.load(line)
            if obj == None:
                ob.sendLine('☞ 그런 무림인은 아애 존재하지 않습니다.')
                return
        
        if obj['소속'] != ob['소속']:
            ob.sendLine('☞ 당신의 소속이 아닙니다.')
            return

        g = GUILD[ob['소속']]
        g['%s리스트' % obj['직위']].remove(obj['이름'])
        obj.attr.__delitem__('소속')
        obj.attr.__delitem__('직위')
        if obj['방파별호'] != '':
            obj.attr.__delitem__('방파별호')
        obj.save(False)
        g['방파원수'] -= 1
        GUILD.save()

        msg = '%s %s 방파에서 파문시킴을 선포합니다.' % (ob.han_iga(), obj.han_obj())
        obj.sendLine('\r\n당신은 파문되었습니다.')
        obj.lpPrompt()
        ob.sendGroup(msg, prompt = True)
        
